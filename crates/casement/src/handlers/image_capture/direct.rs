//! Capture drawn by the GPU straight into a client's dma-buf (wave-2026-10-15/dmabuf): no CPU
//! copy of the frame. Which frames go this way is `casement_backend::capture_path::direct`
//! (the buffer's device and whether the renderer binds its format); every direct draw that fails falls back to the
//! read-back path, which `dma.rs::deliver` answers for any buffer.
//!
//! * An output's frame is already a GPU image (the headless GL canvas, or the display GPU's
//!   capture scene on a real seat) and is blitted: [`output`].
//! * A window is rendered alone into the buffer: [`window`].
//! * Space pictures, cursor images and a software renderer's frames are small or already on the
//!   CPU and keep the read-back path (FINDINGS `wave-2026-10-15/dmabuf`).

use crate::state::State;
use casement_backend::capture_path::direct;
use casement_backend::{BackendError, BackendKind};
use casement_core::OutputName;
use casement_core::geometry::{PhysicalRect, PhysicalSize};
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

/// The device the renderer draws on, when the backend can tell.
fn drawing_device(state: &State) -> Option<u64> {
    match &state.backend {
        BackendKind::Tty(tty) => tty.capture_device(),
        _ => None,
    }
}

/// Whether the backend's renderer binds `wanted` for a capture of `subject` into a dma-buf.
fn binds(state: &State, subject: Subject<'_>, wanted: Format) -> bool {
    match (&state.backend, subject) {
        (BackendKind::Headless(backend), _) => backend.binds_format(wanted),
        (BackendKind::Tty(tty), Subject::Output(name)) => tty.output_binds_format(name, wanted),
        (BackendKind::Tty(tty), Subject::Window) => tty.binds_format(wanted),
        _ => false,
    }
}

/// Whether a frame of `subject` is drawn straight into `buffer` (`false`: read back; a shared
/// memory buffer always is).
pub(in crate::handlers) fn is_direct(
    state: &State,
    buffer: &WlBuffer,
    subject: Subject<'_>,
) -> bool {
    let Ok(held) = get_dmabuf(buffer) else {
        return false;
    };
    let wanted = state.wl.dmabuf.main_device().map(|device| device.0);
    direct(
        wanted,
        drawing_device(state),
        binds(state, subject, held.format()),
    )
}

/// Draws `from`, a part of the frame `name` last presented, into the dma-buf `buffer`.
pub(in crate::handlers) fn output(
    state: &mut State,
    buffer: &WlBuffer,
    name: &OutputName,
    cursor: bool,
    from: PhysicalRect,
) -> Result<(), BackendError> {
    let mut target = held(buffer)?;
    state
        .backend
        .driver_mut()
        .blit_output_dmabuf(name, cursor, from, &mut target)
}

/// Renders `source` (a window, alone) of `size` at `scale` into the dma-buf `buffer`. `true`
/// when the buffer holds the frame; `false` when the path was the read-back or the draw failed:
/// read the frame back.
pub(in crate::handlers) fn window(
    state: &mut State,
    buffer: &WlBuffer,
    source: &dyn casement_backend::headless::FrameSource,
    size: PhysicalSize,
    scale: Scale,
) -> bool {
    if !is_direct(state, buffer, Subject::Window) {
        return false;
    }
    let drawn = held(buffer).and_then(|mut target| {
        state
            .backend
            .driver_mut()
            .render_window_dmabuf(&mut target, size, scale, source)
    });
    match drawn {
        Ok(()) => true,
        Err(error) => {
            tracing::info!(%error, "a window was not drawn into the dma-buf; reading it back");
            false
        }
    }
}

/// The dma-buf behind `buffer`.
fn held(buffer: &WlBuffer) -> Result<Dmabuf, BackendError> {
    get_dmabuf(buffer)
        .cloned()
        .map_err(|_| BackendError::NoFrame)
}
