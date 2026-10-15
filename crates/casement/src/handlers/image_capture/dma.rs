//! Dma-buf capture: what a session offers, and putting a frame into (or reading a cursor out of)
//! a client's buffer, shared memory or dma-buf alike.
//!
//! The offer is the renderer's own set (what the backend's renderer can draw a frame into)
//! meeting what the dma-buf global accepts, so a client can always create the buffer it
//! allocates. On a real seat the offer starts with the display GPU's formats and modifiers and
//! the GPU's renderer draws the pixels; the software renderer (linear buffers) is the fallback
//! (`casement_backend::tty::TtyBackend::write_dmabuf`). The pixels are drawn by
//! `casement_render::pass::write_dmabuf`; shared memory is written directly.

use crate::state::State;
use casement_backend::{BackendError, BackendKind};
use casement_core::geometry::PhysicalSize;
use casement_render::pass::{ArgbImage, CapturedFrame, ReadError, ShmWriteError, write_frame};
use casement_wayland::image_capture::dma_offer;
use smithay::backend::allocator::{Buffer as AllocBuffer, Format, Fourcc};
use smithay::reexports::drm::node::DrmNode;
use smithay::reexports::wayland_server::protocol::wl_buffer::WlBuffer;
use smithay::wayland::dmabuf::get_dmabuf;
use smithay::wayland::image_copy_capture::DmabufConstraints;
use smithay::wayland::shm::with_buffer_contents;

/// Why a frame could not be put into a client's buffer.
#[derive(Debug, thiserror::Error)]
pub(in crate::handlers) enum DeliverError {
    /// The shared-memory buffer refused it.
    #[error("shared memory: {0}")]
    Shm(#[from] ShmWriteError),
    /// The renderer could not draw it into the dma-buf.
    #[error("dma-buf: {0}")]
    Dmabuf(#[from] BackendError),
    /// This backend has no renderer to draw a dma-buf with.
    #[error("this backend draws no dma-buf captures")]
    NoRenderer,
}

/// The formats the backend's renderer draws a capture into.
fn renderable(state: &State) -> Vec<Format> {
    match &state.backend {
        BackendKind::Headless(backend) => backend.capture_formats(),
        BackendKind::Tty(tty) => tty.capture_formats(),
        _ => Vec::new(),
    }
}

/// The dma-buf formats a capture may use now: the renderer's, among those the global accepts.
fn formats(state: &State) -> Vec<(Fourcc, Vec<smithay::backend::allocator::Modifier>)> {
    dma_offer(&renderable(state), &state.wl.dmabuf.accepted())
}

/// What a capture session offers for dma-buf: the main device and the formats; none while no
/// dma-buf is offered to clients, the device cannot be typed as a DRM node or the renderer
/// draws into none of the offered formats (the session then offers shared memory only).
pub(in crate::handlers) fn offer(state: &State) -> Option<DmabufConstraints> {
    let device = state.wl.dmabuf.main_device()?;
    let node = DrmNode::from_dev_id(device.0).ok()?;
    let formats = formats(state);
    (!formats.is_empty()).then_some(DmabufConstraints { node, formats })
}

/// The formats a `wlr` copy is told it may use (no modifiers on that wire).
pub(in crate::handlers) fn fourccs(state: &State) -> Vec<Fourcc> {
    formats(state).into_iter().map(|(code, _)| code).collect()
}

/// Puts `frame` into `buffer`: written directly into shared memory, or drawn by the backend's
/// renderer into a dma-buf.
pub(in crate::handlers) fn deliver(
    state: &mut State,
    buffer: &WlBuffer,
    frame: &CapturedFrame,
) -> Result<(), DeliverError> {
    let Ok(held) = get_dmabuf(buffer) else {
        return Ok(write_frame(buffer, frame)?);
    };
    let mut target = held.clone();
    match &mut state.backend {
        BackendKind::Headless(backend) => backend.write_dmabuf(&mut target, frame)?,
        BackendKind::Tty(tty) => tty.write_dmabuf(&mut target, frame)?,
        _ => return Err(DeliverError::NoRenderer),
    }
    Ok(())
}

/// The size of a capture buffer, shared memory or dma-buf, in pixels.
pub(in crate::handlers) fn buffer_size(buffer: &WlBuffer) -> Option<PhysicalSize> {
    let (w, h) = match get_dmabuf(buffer) {
        Ok(held) => {
            let size = held.size();
            (size.w, size.h)
        }
        Err(_) => with_buffer_contents(buffer, |_, _, data| (data.width, data.height)).ok()?,
    };
    Some(PhysicalSize {
        w: u32::try_from(w).ok()?,
        h: u32::try_from(h).ok()?,
    })
}

/// Why a client's cursor buffer could not be read.
#[derive(Debug, thiserror::Error)]
pub(in crate::handlers) enum CursorReadError {
    /// A shared-memory buffer that cannot be read.
    #[error("shared memory: {0}")]
    Shm(#[from] ReadError),
    /// A dma-buf the renderer could not read back.
    #[error("dma-buf: {0}")]
    Dmabuf(#[from] BackendError),
    /// This backend has no renderer to read a dma-buf with.
    #[error("this backend reads no dma-bufs")]
    NoRenderer,
}

/// The pixels of the client's cursor `buffer`: shared memory directly, a dma-buf through the
/// renderer.
pub(in crate::handlers) fn read_cursor(
    state: &mut State,
    buffer: &WlBuffer,
) -> Result<ArgbImage, CursorReadError> {
    let Ok(held) = get_dmabuf(buffer) else {
        return Ok(ArgbImage::from_shm(buffer)?);
    };
    let source = held.clone();
    match &mut state.backend {
        BackendKind::Headless(backend) => Ok(backend.read_dmabuf(&source)?),
        BackendKind::Tty(tty) => Ok(tty.read_dmabuf(&source)?),
        _ => Err(CursorReadError::NoRenderer),
    }
}
