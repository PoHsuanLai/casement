//! Buffers and a draw helper for the topology tests (spike SP4, ported): linear ARGB8888
//! udmabufs over a memfd the test keeps, so pixels are checked without smithay.

use smithay::backend::allocator::Fourcc;
use smithay::backend::allocator::dmabuf::Dmabuf;
use smithay::backend::allocator::udmabuf::UdmabufAllocator;
use smithay::backend::drm::DrmNode;
use smithay::backend::renderer::{Bind, Color32F, Frame, Renderer};
use smithay::utils::{Buffer as BufferCoords, Physical, Rectangle, Size, Transform};
use std::fs::File;
use std::os::unix::fs::FileExt;

/// One linear ARGB8888 buffer backed by a memfd the test keeps.
pub struct TestBuffer {
    file: File,
    pub dmabuf: Dmabuf,
    pub w: u32,
    pub h: u32,
    stride: u32,
}

impl TestBuffer {
    /// A `w` x `h` buffer, tagged with `node` (A5: an untagged buffer silently skips the copy).
    pub fn new(w: u32, h: u32, node: Option<DrmNode>) -> std::io::Result<TestBuffer> {
        let stride = (w * 4).next_multiple_of(256);
        let size = (u64::from(stride) * u64::from(h)).next_multiple_of(4096);
        let fd = rustix::fs::memfd_create(
            "casement-topology",
            rustix::fs::MemfdFlags::ALLOW_SEALING | rustix::fs::MemfdFlags::CLOEXEC,
        )?;
        rustix::fs::ftruncate(&fd, size)?;
        rustix::fs::fcntl_add_seals(&fd, rustix::fs::SealFlags::SHRINK)?;
        let dmabuf = UdmabufAllocator::new()?.create_buffer_from_memfd(
            &fd,
            0,
            size as usize,
            Fourcc::Argb8888,
            w,
            h,
            stride,
        )?;
        dmabuf.set_node(node);
        Ok(TestBuffer {
            file: File::from(fd),
            dmabuf,
            w,
            h,
            stride,
        })
    }

    /// Writes the pattern of `frame`.
    pub fn fill(&self, frame: u32) -> std::io::Result<()> {
        let mut row = vec![0u8; self.stride as usize];
        for y in 0..self.h {
            for x in 0..self.w {
                let at = (x * 4) as usize;
                row[at..at + 4].copy_from_slice(&pattern(x, y, frame).to_le_bytes());
            }
            self.file
                .write_all_at(&row, u64::from(y) * u64::from(self.stride))?;
        }
        Ok(())
    }

    /// How many pixels differ from the pattern of `frame`.
    pub fn mismatches(&self, frame: u32) -> std::io::Result<u64> {
        let mut row = vec![0u8; (self.w * 4) as usize];
        let mut bad = 0;
        for y in 0..self.h {
            self.file
                .read_exact_at(&mut row, u64::from(y) * u64::from(self.stride))?;
            for x in 0..self.w {
                let at = (x * 4) as usize;
                let mut px = [0u8; 4];
                px.copy_from_slice(&row[at..at + 4]);
                if u32::from_le_bytes(px) != pattern(x, y, frame) {
                    bad += 1;
                }
            }
        }
        Ok(bad)
    }

    /// The pattern of `frame` as shm bytes (tightly packed ARGB8888).
    pub fn shm_bytes(w: u32, h: u32, frame: u32) -> Vec<u8> {
        (0..h)
            .flat_map(|y| (0..w).flat_map(move |x| pattern(x, y, frame).to_le_bytes()))
            .collect()
    }
}

/// A coordinate-encoding pattern: R = x low, G = y low, B = x and y high nibbles xor the frame.
pub fn pattern(x: u32, y: u32, frame: u32) -> u32 {
    let b = ((((x >> 8) & 0xf) << 4) | ((y >> 8) & 0xf)) ^ (frame.wrapping_mul(37) & 0xff);
    0xff00_0000 | ((x & 0xff) << 16) | ((y & 0xff) << 8) | b
}

fn full<K>(w: u32, h: u32) -> Rectangle<i32, K> {
    Rectangle::from_size(Size::from((w as i32, h as i32)))
}

/// Renders `texture` over the whole of `out` through `renderer`, and waits.
pub fn draw<R>(renderer: &mut R, out: &mut TestBuffer, texture: &R::TextureId) -> Result<(), String>
where
    R: Renderer + Bind<Dmabuf>,
{
    let (w, h) = (out.w, out.h);
    let mut target = renderer
        .bind(&mut out.dmabuf)
        .map_err(|error| format!("bind: {error:?}"))?;
    let mut frame = renderer
        .render(
            &mut target,
            Size::<i32, Physical>::from((w as i32, h as i32)),
            Transform::Normal,
        )
        .map_err(|error| format!("render: {error:?}"))?;
    let all: Rectangle<i32, Physical> = full(w, h);
    frame
        .clear(Color32F::BLACK, &[all])
        .map_err(|error| format!("clear: {error:?}"))?;
    frame
        .render_texture_from_to(
            texture,
            full::<BufferCoords>(w, h).to_f64(),
            all,
            &[all],
            &[],
            Transform::Normal,
            1.0,
        )
        .map_err(|error| format!("draw: {error:?}"))?;
    let sync = frame
        .finish()
        .map_err(|error| format!("finish: {error:?}"))?;
    let _ = sync.wait();
    Ok(())
}
