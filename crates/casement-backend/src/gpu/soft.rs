//! The soft-GPU pieces (SP4, A4).
//!
//! * [`SoftPixman`]: smithay's `PixmanRenderer` shares one static `ContextId` across every
//!   instance, so two pixman "GPUs" collapse into one slot in `MultiTexture` and the
//!   cross-device copy panics. This wrapper gives each instance its own id and counts which
//!   copy path ran (A5: tests assert the path, untagged buffers silently skip the copy).
//! * [`FixedUdmabuf`]: smithay's `UdmabufAllocator::create_buffer` computes the stride from bits
//!   per pixel (8x over-allocation, EINVAL above about 1080p at the kernel's 64 MB limit); this
//!   allocator makes a memfd with stride = width x bytes per pixel and hands it to
//!   `create_buffer_from_memfd`. `UdmabufAllocator::create_buffer` is banned by the boundary
//!   check.
//!
//! Neither is reported upstream (A13); both are QUIRKS rows. The renderer and allocator trait
//! impls are pure delegation (`RendererSuper`, `Renderer`, `ImportMem`, `ImportDma`,
//! `ExportMem`, `Bind<Dmabuf>`, a wrapped [`SoftFrame`]; `Allocator`); `Offscreen` is not
//! implemented, SP4's render-on-B run did not need it. The `GraphicsApi` that enumerates the
//! fabricated `/sys/dev/char/226:(128+n)` nodes is the test kit's.

use crate::backend::BackendError;
use rustix::fs::{MemfdFlags, SealFlags};
use smithay::backend::allocator::dmabuf::Dmabuf;
use smithay::backend::allocator::format::{FormatSet, get_bpp};
use smithay::backend::allocator::udmabuf::{STRIDE_ALIGN, UdmabufAllocator};
use smithay::backend::allocator::{Allocator, Fourcc, Modifier};
use smithay::backend::renderer::pixman::{
    PixmanError, PixmanFrame, PixmanMapping, PixmanRenderer, PixmanTarget, PixmanTexture,
};
use smithay::backend::renderer::sync::SyncPoint;
use smithay::backend::renderer::{
    Bind, Color32F, ContextId, DebugFlags, ExportMem, Frame, ImportDma, ImportMem, Renderer,
    RendererSuper, TextureFilter,
};
use smithay::utils::{Buffer as BufferCoords, Physical, Rectangle, Size, Transform};
use std::io;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// How often each copy path ran on a soft GPU.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CopyCounters {
    /// `import_dmabuf` calls.
    pub import_dmabuf: u32,
    /// `import_memory` calls.
    pub import_memory: u32,
    /// `copy_texture` calls.
    pub copy_texture: u32,
    /// `bind` of a dma-buf target.
    pub bind_dmabuf: u32,
}

/// A shared view of one soft GPU's counters: the renderer sits inside smithay's `GpuManager`
/// once it is enumerated, and a test reads the counts through this.
///
/// ```
/// use casement_backend::gpu::SoftPixman;
///
/// let gpu = SoftPixman::new().expect("pixman");
/// assert_eq!(gpu.counter_handle().snapshot(), gpu.counters());
/// ```
#[derive(Debug, Clone, Default)]
pub struct CounterHandle(Arc<Mutex<CopyCounters>>);

impl CounterHandle {
    /// The counts so far.
    pub fn snapshot(&self) -> CopyCounters {
        *self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn bump(&self, count: impl FnOnce(&mut CopyCounters) -> &mut u32) {
        let mut counters = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *count(&mut counters) += 1;
    }
}

/// A pixman renderer with its own `ContextId`.
#[derive(Debug)]
pub struct SoftPixman {
    inner: PixmanRenderer,
    context: ContextId<PixmanTexture>,
    counters: CounterHandle,
}

impl SoftPixman {
    /// A new soft GPU renderer with a fresh context id.
    pub fn new() -> Result<SoftPixman, BackendError> {
        let inner = PixmanRenderer::new()?;
        Ok(SoftPixman {
            inner,
            context: ContextId::new(),
            counters: CounterHandle::default(),
        })
    }

    /// The copy-path counters so far.
    pub fn counters(&self) -> CopyCounters {
        self.counters.snapshot()
    }

    /// A handle that keeps reading this renderer's counters after it moves.
    pub fn counter_handle(&self) -> CounterHandle {
        self.counters.clone()
    }
}

/// The frame of a [`SoftPixman`]: pixman's, under this renderer's context id.
#[derive(Debug)]
pub struct SoftFrame<'frame, 'buffer> {
    inner: PixmanFrame<'frame, 'buffer>,
    context: ContextId<PixmanTexture>,
}

impl RendererSuper for SoftPixman {
    type Error = PixmanError;
    type TextureId = PixmanTexture;
    type Framebuffer<'buffer> = PixmanTarget<'buffer>;
    type Frame<'frame, 'buffer>
        = SoftFrame<'frame, 'buffer>
    where
        'buffer: 'frame,
        Self: 'frame;
}

impl Renderer for SoftPixman {
    fn context_id(&self) -> ContextId<PixmanTexture> {
        self.context.clone()
    }

    fn downscale_filter(&mut self, filter: TextureFilter) -> Result<(), PixmanError> {
        self.inner.downscale_filter(filter)
    }

    fn upscale_filter(&mut self, filter: TextureFilter) -> Result<(), PixmanError> {
        self.inner.upscale_filter(filter)
    }

    fn set_debug_flags(&mut self, flags: DebugFlags) {
        self.inner.set_debug_flags(flags);
    }

    fn debug_flags(&self) -> DebugFlags {
        self.inner.debug_flags()
    }

    fn render<'frame, 'buffer>(
        &'frame mut self,
        framebuffer: &'frame mut PixmanTarget<'buffer>,
        output_size: Size<i32, Physical>,
        dst_transform: Transform,
    ) -> Result<SoftFrame<'frame, 'buffer>, PixmanError>
    where
        'buffer: 'frame,
    {
        let context = self.context.clone();
        let inner = self.inner.render(framebuffer, output_size, dst_transform)?;
        Ok(SoftFrame { inner, context })
    }

    fn wait(&mut self, sync: &SyncPoint) -> Result<(), PixmanError> {
        self.inner.wait(sync)
    }

    fn cleanup_texture_cache(&mut self) -> Result<(), PixmanError> {
        self.inner.cleanup_texture_cache()
    }

    fn invalidate_caches(&mut self) -> Result<(), PixmanError> {
        self.inner.invalidate_caches()
    }
}

impl Frame for SoftFrame<'_, '_> {
    type Error = PixmanError;
    type TextureId = PixmanTexture;

    fn context_id(&self) -> ContextId<PixmanTexture> {
        self.context.clone()
    }

    fn clear(
        &mut self,
        color: Color32F,
        at: &[Rectangle<i32, Physical>],
    ) -> Result<(), PixmanError> {
        self.inner.clear(color, at)
    }

    fn draw_solid(
        &mut self,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
        color: Color32F,
    ) -> Result<(), PixmanError> {
        self.inner.draw_solid(dst, damage, color)
    }

    fn render_texture_from_to(
        &mut self,
        texture: &PixmanTexture,
        src: Rectangle<f64, BufferCoords>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
        opaque_regions: &[Rectangle<i32, Physical>],
        src_transform: Transform,
        alpha: f32,
    ) -> Result<(), PixmanError> {
        self.inner.render_texture_from_to(
            texture,
            src,
            dst,
            damage,
            opaque_regions,
            src_transform,
            alpha,
        )
    }

    fn transformation(&self) -> Transform {
        self.inner.transformation()
    }

    fn output_size(&self) -> Size<i32, Physical> {
        self.inner.output_size()
    }

    fn wait(&mut self, sync: &SyncPoint) -> Result<(), PixmanError> {
        self.inner.wait(sync)
    }

    fn finish(self) -> Result<SyncPoint, PixmanError> {
        self.inner.finish()
    }
}

impl ImportMem for SoftPixman {
    fn import_memory(
        &mut self,
        data: &[u8],
        format: Fourcc,
        size: Size<i32, BufferCoords>,
        flipped: bool,
    ) -> Result<PixmanTexture, PixmanError> {
        self.counters.bump(|counters| &mut counters.import_memory);
        self.inner.import_memory(data, format, size, flipped)
    }

    fn update_memory(
        &mut self,
        texture: &PixmanTexture,
        data: &[u8],
        region: Rectangle<i32, BufferCoords>,
    ) -> Result<(), PixmanError> {
        self.inner.update_memory(texture, data, region)
    }

    fn mem_formats(&self) -> Box<dyn Iterator<Item = Fourcc>> {
        self.inner.mem_formats()
    }
}

impl ImportDma for SoftPixman {
    fn import_dmabuf(
        &mut self,
        dmabuf: &Dmabuf,
        damage: Option<&[Rectangle<i32, BufferCoords>]>,
    ) -> Result<PixmanTexture, PixmanError> {
        self.counters.bump(|counters| &mut counters.import_dmabuf);
        self.inner.import_dmabuf(dmabuf, damage)
    }

    fn dmabuf_formats(&self) -> FormatSet {
        self.inner.dmabuf_formats()
    }
}

impl ExportMem for SoftPixman {
    type TextureMapping = PixmanMapping;

    fn copy_framebuffer(
        &mut self,
        target: &PixmanTarget<'_>,
        region: Rectangle<i32, BufferCoords>,
        format: Fourcc,
    ) -> Result<PixmanMapping, PixmanError> {
        self.inner.copy_framebuffer(target, region, format)
    }

    fn copy_texture(
        &mut self,
        texture: &PixmanTexture,
        region: Rectangle<i32, BufferCoords>,
        format: Fourcc,
    ) -> Result<PixmanMapping, PixmanError> {
        self.counters.bump(|counters| &mut counters.copy_texture);
        self.inner.copy_texture(texture, region, format)
    }

    fn can_read_texture(&mut self, texture: &PixmanTexture) -> Result<bool, PixmanError> {
        self.inner.can_read_texture(texture)
    }

    fn map_texture<'a>(&mut self, mapping: &'a PixmanMapping) -> Result<&'a [u8], PixmanError> {
        self.inner.map_texture(mapping)
    }
}

impl Bind<Dmabuf> for SoftPixman {
    fn bind<'a>(&mut self, target: &'a mut Dmabuf) -> Result<PixmanTarget<'a>, PixmanError> {
        self.counters.bump(|counters| &mut counters.bind_dmabuf);
        self.inner.bind(target)
    }

    fn supported_formats(&self) -> Option<FormatSet> {
        Bind::<Dmabuf>::supported_formats(&self.inner)
    }
}

/// A dma-buf allocator over memfds with the right stride.
#[derive(Debug)]
pub struct FixedUdmabuf {
    udmabuf: UdmabufAllocator,
}

impl FixedUdmabuf {
    /// Opens `/dev/udmabuf` (bound into the jail; preflight exits 3 without it).
    pub fn new() -> Result<FixedUdmabuf, BackendError> {
        UdmabufAllocator::new()
            .map(|udmabuf| FixedUdmabuf { udmabuf })
            .map_err(|source| BackendError::DeniedOpen {
                path: PathBuf::from("/dev/udmabuf"),
                source,
            })
    }
}

impl Allocator for FixedUdmabuf {
    type Buffer = Dmabuf;
    type Error = io::Error;

    /// A linear buffer: a sealed memfd with stride = width x bytes per pixel (rounded to
    /// `STRIDE_ALIGN`), made a dma-buf by `create_buffer_from_memfd`.
    fn create_buffer(
        &mut self,
        width: u32,
        height: u32,
        fourcc: Fourcc,
        modifiers: &[Modifier],
    ) -> Result<Dmabuf, io::Error> {
        if !modifiers.contains(&Modifier::Linear) {
            return Err(io::ErrorKind::InvalidData.into());
        }
        let bytes = get_bpp(fourcc).ok_or(io::ErrorKind::InvalidData)? / 8;
        let stride = (width as usize * bytes).next_multiple_of(STRIDE_ALIGN);
        let size = (height as usize * stride).next_multiple_of(4096);
        let fd = rustix::fs::memfd_create(
            "casement-soft-gpu",
            MemfdFlags::ALLOW_SEALING | MemfdFlags::CLOEXEC,
        )?;
        rustix::fs::ftruncate(&fd, size as u64)?;
        rustix::fs::fcntl_add_seals(&fd, SealFlags::SHRINK)?;
        self.udmabuf
            .create_buffer_from_memfd(&fd, 0, size, fourcc, width, height, stride as u32)
    }
}
