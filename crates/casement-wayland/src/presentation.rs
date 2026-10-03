//! `wp_presentation`: when a frame reached the screen. Timestamps are the clock's `Mono`
//! (virtual in tests), never the wall clock.

use crate::states::{GlobalFilter, WaylandFault};
use casement_core::time::{Millihertz, Mono, VblankSeq};
use casement_core::{Event, OutputKey};
use smithay::reexports::wayland_protocols::wp::presentation_time::server::{
    wp_presentation::WpPresentation, wp_presentation_feedback::WpPresentationFeedback,
};
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::{Dispatch, DisplayHandle, GlobalDispatch};
use smithay::wayland::GlobalData;
use smithay::wayland::presentation::{PresentationData, PresentationState};

/// The `clk_id` casement advertises: `CLOCK_MONOTONIC` (the session's `Mono` counts from a
/// monotonic epoch; the virtual clock stands in for it in tests).
pub const CLOCK_MONOTONIC: u32 = 1;

/// `wp_presentation`.
#[derive(Debug)]
pub struct PresentationPart {
    /// smithay's state.
    pub state: PresentationState,
}

impl PresentationPart {
    /// Creates the global (version 2). Every connection sees it.
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> PresentationPart
    where
        D: GlobalDispatch<WpPresentation, PresentationData>
            + Dispatch<WpPresentation, PresentationData>
            + Dispatch<WpPresentationFeedback, GlobalData>
            + 'static,
    {
        let _ = role_filter;
        PresentationPart {
            state: PresentationState::new::<D>(display, CLOCK_MONOTONIC),
        }
    }
}

/// One presented frame on one output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Presented {
    /// The vblank it was shown at.
    pub seq: VblankSeq,
    /// When.
    pub at: Mono,
    /// The output's refresh then.
    pub refresh: Millihertz,
}

/// A committed surface asked for presentation feedback: it is collected for the next frame of
/// the outputs the surface is on.
pub fn on_feedback(part: &mut PresentationPart, surface: &WlSurface) -> Vec<Event> {
    let _ = (part, surface);
    todo!("P0-H1: presentation::on_feedback")
}

/// Sends `presented` to every feedback collected for `output`'s last frame.
pub fn apply_presented(
    part: &mut PresentationPart,
    output: &OutputKey,
    presented: Presented,
) -> Result<(), WaylandFault> {
    let _ = (part, output, presented);
    todo!("P0-H1: presentation::apply_presented")
}
