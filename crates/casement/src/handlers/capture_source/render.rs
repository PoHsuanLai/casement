//! Drawing one window into a client's buffer: its surface tree alone. A dma-buf the GPU can draw
//! into gets it rendered straight in; any other buffer is filled from a read-back through the
//! backend's software renderer (the headless backend's own, or the tty seat's pixman shadow).

use crate::handlers::image_capture::{Drawn, draw_window_direct};
use crate::state::State;
use casement_backend::BackendKind;
use casement_backend::headless::{FrameSource, RgbaFrame};
use casement_core::WindowId;
use casement_core::geometry::{LogicalSize, PhysicalRect, PhysicalSize};
use casement_core::time::Mono;
use casement_render::pass::{CapturedFrame, window_capture_size, window_tree};
use casement_render::{CasementElement, Compositor};
use casement_wayland::capture_source::ToplevelCaptureRequest;
use casement_wayland::image_capture::CaptureFailure;
use ds_core::geometry::scale::Scale;
use smithay::backend::renderer::gles::GlesRenderer;
use smithay::backend::renderer::pixman::PixmanRenderer;
#[cfg(feature = "testing")]
use smithay::backend::renderer::test::DummyRenderer;
use smithay::reexports::wayland_server::protocol::wl_buffer::WlBuffer;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;

/// One window's surface tree, ready to be turned into elements for whichever renderer draws it.
struct WindowSource {
    root: WlSurface,
    content: LogicalSize,
    scale: Scale,
}

impl WindowSource {
    fn elements<R: Compositor>(&self, renderer: &mut R) -> Vec<CasementElement<R>> {
        window_tree(&self.root, self.content, self.scale, renderer)
    }
}

impl FrameSource for WindowSource {
    fn pixman(&self, renderer: &mut PixmanRenderer) -> Vec<CasementElement<PixmanRenderer>> {
        self.elements(renderer)
    }

    fn gles(&self, renderer: &mut GlesRenderer) -> Vec<CasementElement<GlesRenderer>> {
        self.elements(renderer)
    }

    #[cfg(feature = "testing")]
    fn dummy(&self, renderer: &mut DummyRenderer) -> Vec<CasementElement<DummyRenderer>> {
        self.elements(renderer)
    }
}

/// `window`'s surface tree and the size and scale it is drawn at: what both the read-back
/// and the direct draw render. `OutputGone` when the window is not drawn or has no output.
fn window_source(
    state: &State,
    window: WindowId,
) -> Result<(WindowSource, PhysicalSize), CaptureFailure> {
    let (content, scale) =
        super::content_and_scale(state, window).ok_or(CaptureFailure::OutputGone)?;
    let size = window_capture_size(content, scale);
    let root = state
        .render
        .textures
        .window(&window)
        .map(|entry| entry.root.clone())
        .ok_or(CaptureFailure::OutputGone)?;
    Ok((
        WindowSource {
            root,
            content,
            scale,
        },
        size,
    ))
}

/// `window` drawn alone at the scale of its output: its content size, RGBA rows top first,
/// transparent where it does not draw. What window capture sends and `casementctl shot
/// --window` writes. `OutputGone` when the window is not drawn or has no output.
pub(crate) fn window_rgba(
    state: &mut State,
    window: WindowId,
) -> Result<RgbaFrame, CaptureFailure> {
    let (source, size) = window_source(state, window)?;
    let drawn = match &mut state.backend {
        BackendKind::Headless(backend) => backend.render_offscreen(size, source.scale, &source),
        BackendKind::Tty(tty) => tty.render_offscreen(size, source.scale, &source),
        _ => return Err(CaptureFailure::Renderer),
    };
    drawn.map_err(|error| {
        tracing::warn!(%error, "a window was not drawn for capture");
        CaptureFailure::Renderer
    })
}

/// Draws the window of `request` into `buffer` and says when. The window's size or scale may
/// have changed since the request was made: then the picture no longer fits the buffer and the
/// frame fails with `buffer_constraints`.
pub(super) fn draw_window(
    state: &mut State,
    request: &ToplevelCaptureRequest,
    buffer: &WlBuffer,
) -> Result<Mono, CaptureFailure> {
    let (source, size) = window_source(state, request.window)?;
    if size != request.size {
        return Err(CaptureFailure::BufferMismatch);
    }
    // A dma-buf the GPU can draw into gets the window rendered straight into it; anything else,
    // or a draw that failed, goes through the read-back.
    if draw_window_direct(state, buffer, &source, size, source.scale) == Drawn::Direct {
        return Ok(state.clock.now());
    }
    let drawn = window_rgba(state, request.window)?;
    if drawn.size != request.size {
        return Err(CaptureFailure::BufferMismatch);
    }
    let area = PhysicalRect {
        x: 0,
        y: 0,
        w: drawn.size.w,
        h: drawn.size.h,
    };
    let captured = CapturedFrame::from_rgba(area, &drawn.bytes);
    crate::handlers::image_capture::deliver(state, buffer, &captured).map_err(|error| {
        tracing::warn!(%error, "a window capture frame was not written");
        CaptureFailure::BufferMismatch
    })?;
    Ok(state.clock.now())
}
