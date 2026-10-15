//! Capture drawn by the GPU straight into a client's dma-buf (wave-2026-10-15/dmabuf): no CPU
//! copy of the frame. Which frames go this way is `casement_backend::capture_path::capture_path`
//! (the buffer's kind and the renderer's class); every direct draw that fails falls back to the
//! read-back path, which `dma.rs::deliver` answers for any buffer.
//!
//! * An output's frame is already a GPU image (the headless GL canvas, or the display GPU's
//!   capture scene on a real seat) and is blitted: [`output`].
//! * A window is rendered alone into the buffer: [`window`].
//! * Space pictures, cursor images and a software renderer's frames are small or already on the
//!   CPU and keep the read-back path (FINDINGS `wave-2026-10-15/dmabuf`).

use crate::state::State;
use casement_backend::capture_path::placement;
use casement_backend::capture_path::{BufferKind, CapturePath, RendererClass, capture_path};
use casement_backend::{BackendError, BackendKind};
use casement_core::OutputName;
use casement_core::geometry::{PhysicalRect, PhysicalSize};
use casement_render::pass::CursorInclusion;
use casement_wayland::image_capture::CursorPaint;
use ds_core::geometry::scale::Scale;
use smithay::backend::allocator::dmabuf::Dmabuf;
use smithay::backend::allocator::{Buffer as AllocBuffer, Format};
use smithay::reexports::wayland_server::protocol::wl_buffer::WlBuffer;
use smithay::wayland::dmabuf::get_dmabuf;

/// What a path is chosen for.
#[derive(Debug, Clone, Copy)]
pub(in crate::handlers) enum Subject<'a> {
    /// A frame an output last presented.
    Output(&'a OutputName),
    /// A window drawn alone.
    Window,
}

/// Whether the direct draw drew.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::handlers) enum Drawn {
    /// The buffer holds the frame.
    Direct,
    /// The path was the read-back, or the direct draw failed: read the frame back.
    ReadBack,
}

/// `paint` as the render crate says it.
pub(in crate::handlers) fn inclusion(paint: CursorPaint) -> CursorInclusion {
    match paint {
        CursorPaint::Hidden => CursorInclusion::Exclude,
        _ => CursorInclusion::Include,
    }
}

/// The device the renderer draws on, when the backend can tell.
fn drawing_device(state: &State) -> Option<u64> {
    match &state.backend {
        BackendKind::Tty(tty) => tty.capture_device(),
        _ => None,
    }
}

/// What the backend's renderer is to a capture of `subject` into a dma-buf of `wanted`.
fn class(state: &State, subject: Subject<'_>, wanted: Format) -> RendererClass {
    match (&state.backend, subject) {
        (BackendKind::Headless(backend), _) => backend.capture_class(wanted),
        (BackendKind::Tty(tty), Subject::Output(name)) => tty.output_class(name, wanted),
        (BackendKind::Tty(tty), Subject::Window) => tty.capture_class(wanted),
        _ => RendererClass::Software,
    }
}

/// How a frame of `subject` reaches `buffer`.
pub(in crate::handlers) fn path_of(
    state: &State,
    buffer: &WlBuffer,
    subject: Subject<'_>,
) -> CapturePath {
    let Ok(held) = get_dmabuf(buffer) else {
        return capture_path(BufferKind::Shm, RendererClass::Software);
    };
    let wanted = state.wl.dmabuf.main_device().map(|device| device.0);
    let kind = BufferKind::Dmabuf(placement(wanted, drawing_device(state)));
    capture_path(kind, class(state, subject, held.format()))
}

/// Draws `from`, a part of the frame `name` last presented, into the dma-buf `buffer`.
pub(in crate::handlers) fn output(
    state: &mut State,
    buffer: &WlBuffer,
    name: &OutputName,
    paint: CursorPaint,
    from: PhysicalRect,
) -> Result<(), BackendError> {
    let mut target = held(buffer)?;
    let cursor = inclusion(paint);
    match &mut state.backend {
        BackendKind::Headless(backend) => {
            backend.blit_output_dmabuf(name, cursor, from, &mut target)
        }
        BackendKind::Tty(tty) => tty.blit_output_dmabuf(name, cursor, from, &mut target),
        _ => Err(BackendError::NoFrame),
    }
}

/// Renders `source` (a window, alone) of `size` at `scale` into the dma-buf `buffer`.
pub(in crate::handlers) fn window(
    state: &mut State,
    buffer: &WlBuffer,
    source: &dyn casement_backend::headless::FrameSource,
    size: PhysicalSize,
    scale: Scale,
) -> Drawn {
    if path_of(state, buffer, Subject::Window) != CapturePath::Direct {
        return Drawn::ReadBack;
    }
    let drawn = held(buffer).and_then(|mut target| match &mut state.backend {
        BackendKind::Headless(backend) => {
            backend.render_window_dmabuf(&mut target, size, scale, source)
        }
        BackendKind::Tty(tty) => tty.render_window_dmabuf(&mut target, size, scale, source),
        _ => Err(BackendError::NoFrame),
    });
    match drawn {
        Ok(()) => Drawn::Direct,
        Err(error) => {
            tracing::info!(%error, "a window was not drawn into the dma-buf; reading it back");
            Drawn::ReadBack
        }
    }
}

/// The dma-buf behind `buffer`.
fn held(buffer: &WlBuffer) -> Result<Dmabuf, BackendError> {
    get_dmabuf(buffer)
        .map(Clone::clone)
        .map_err(|_| BackendError::NoFrame)
}
