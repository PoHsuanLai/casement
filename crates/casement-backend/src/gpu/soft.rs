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
//! impls (pure delegation: `RendererSuper`, `Renderer`, `ImportMem`, `ImportDma`, `ExportMem`,
//! `Bind<Dmabuf>`, `Offscreen`, a wrapped `Frame`; `Allocator`) are P1-D4's, with the
//! `GraphicsApi` that enumerates the jail's fabricated `/sys/dev/char/226:(128+n)` nodes.

use crate::backend::BackendError;
use smithay::backend::allocator::udmabuf::UdmabufAllocator;
use smithay::backend::renderer::ContextId;
use smithay::backend::renderer::pixman::{PixmanRenderer, PixmanTexture};

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

/// A pixman renderer with its own `ContextId`.
#[derive(Debug)]
pub struct SoftPixman {
    #[expect(dead_code, reason = "P1-D4: the wrapped renderer")]
    inner: PixmanRenderer,
    #[expect(dead_code, reason = "P1-D4: this instance's own context id")]
    context: ContextId<PixmanTexture>,
    counters: CopyCounters,
}

impl SoftPixman {
    /// A new soft GPU renderer with a fresh context id.
    pub fn new() -> Result<SoftPixman, BackendError> {
        let inner =
            PixmanRenderer::new().map_err(|error| BackendError::Renderer(error.to_string()))?;
        Ok(SoftPixman {
            inner,
            context: ContextId::new(),
            counters: CopyCounters::default(),
        })
    }

    /// The copy-path counters so far.
    pub fn counters(&self) -> CopyCounters {
        self.counters
    }
}

/// A dma-buf allocator over memfds with the right stride.
#[derive(Debug)]
pub struct FixedUdmabuf {
    #[expect(dead_code, reason = "P1-D4: create_buffer_from_memfd")]
    udmabuf: UdmabufAllocator,
}

impl FixedUdmabuf {
    /// Opens `/dev/udmabuf` (bound into the jail; preflight exits 3 without it).
    pub fn new() -> Result<FixedUdmabuf, BackendError> {
        UdmabufAllocator::new()
            .map(|udmabuf| FixedUdmabuf { udmabuf })
            .map_err(|error| BackendError::DeniedOpen(format!("/dev/udmabuf: {error}")))
    }
}
