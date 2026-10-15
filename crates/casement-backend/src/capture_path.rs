//! Which way a capture frame reaches the client's buffer (wave-2026-10-15/dmabuf): drawn by
//! the GPU straight into a dma-buf, or read back and copied. One pure function, so the choice
//! is a table and not a set of `if`s at each of the four places that answer a frame.
//!
//! ```
//! use casement_backend::capture_path::{
//!     BufferKind, CapturePath, Import, Placement, RendererClass, capture_path,
//! };
//!
//! let local = BufferKind::Dmabuf(Placement::Local);
//! assert_eq!(
//!     capture_path(local, RendererClass::Gpu(Import::Accepted)),
//!     CapturePath::Direct
//! );
//! assert_eq!(capture_path(BufferKind::Shm, RendererClass::Software), CapturePath::ReadBack);
//! ```

/// What the client gave to be filled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BufferKind {
    /// A shared-memory pool.
    Shm,
    /// A dma-buf, on the device the renderer draws on or on another.
    Dmabuf(Placement),
}

/// Whether a dma-buf lives on the GPU that draws the capture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Placement {
    /// The renderer's own device, or one nobody can tell apart from it.
    Local,
    /// Another device. casement has no renderer that draws across two GPUs for a capture, so
    /// such a buffer is filled from a read-back (FINDINGS `wave-2026-10-15/dmabuf`).
    Foreign,
}

/// Whether the renderer binds the buffer's format and modifier as a render target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Import {
    /// It lists the pair.
    Accepted,
    /// It does not.
    Refused,
}

/// What draws a capture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RendererClass {
    /// A CPU renderer (pixman) or none (the dummy renderer, winit).
    Software,
    /// A GL renderer, which keeps the frame on the GPU.
    Gpu(Import),
}

/// How the frame reaches the buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CapturePath {
    /// The renderer draws (or blits) into the buffer: no CPU copy of the frame.
    Direct,
    /// The frame is read back, then written (shared memory) or uploaded (dma-buf).
    ReadBack,
}

/// The path for a `buffer` and the `renderer` that would fill it. A failed direct draw always
/// falls back to [`CapturePath::ReadBack`], which every buffer has.
pub fn capture_path(buffer: BufferKind, renderer: RendererClass) -> CapturePath {
    match (buffer, renderer) {
        (BufferKind::Dmabuf(Placement::Local), RendererClass::Gpu(Import::Accepted)) => {
            CapturePath::Direct
        }
        _ => CapturePath::ReadBack,
    }
}

/// Where a dma-buf is, given the device clients are told to allocate on (`wanted`) and the
/// device the renderer draws on (`drawing`); `Local` while either is unknown.
pub fn placement(wanted: Option<u64>, drawing: Option<u64>) -> Placement {
    match (wanted, drawing) {
        (Some(wanted), Some(drawing)) if wanted != drawing => Placement::Foreign,
        _ => Placement::Local,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_path_follows_the_buffer_and_the_renderer() {
        use BufferKind::{Dmabuf, Shm};
        use CapturePath::{Direct, ReadBack};
        use Placement::{Foreign, Local};
        use RendererClass::{Gpu, Software};
        // name, buffer, renderer, path
        let table = [
            (
                "dma-buf the gpu lists",
                Dmabuf(Local),
                Gpu(Import::Accepted),
                Direct,
            ),
            (
                "dma-buf the gpu refuses",
                Dmabuf(Local),
                Gpu(Import::Refused),
                ReadBack,
            ),
            (
                "dma-buf, software renderer",
                Dmabuf(Local),
                Software,
                ReadBack,
            ),
            (
                "dma-buf on another gpu",
                Dmabuf(Foreign),
                Gpu(Import::Accepted),
                ReadBack,
            ),
            ("shm, gpu", Shm, Gpu(Import::Accepted), ReadBack),
            ("shm, software", Shm, Software, ReadBack),
        ];
        for (name, buffer, renderer, path) in table {
            assert_eq!(capture_path(buffer, renderer), path, "{name}");
        }
    }

    #[test]
    fn a_buffer_is_foreign_only_when_both_devices_are_known_and_differ() {
        let table = [
            ("same", Some(0xE280), Some(0xE280), Placement::Local),
            ("differ", Some(0xE280), Some(0xE281), Placement::Foreign),
            ("renderer unknown", Some(0xE280), None, Placement::Local),
            ("main unknown", None, Some(0xE280), Placement::Local),
            ("none known", None, None, Placement::Local),
        ];
        for (name, wanted, drawing, expected) in table {
            assert_eq!(placement(wanted, drawing), expected, "{name}");
        }
    }
}
