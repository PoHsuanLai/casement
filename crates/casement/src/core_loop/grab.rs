//! An interactive move or resize: `Effect::StartGrab` opens it with the pointer where it is,
//! each pointer motion until the button is released puts the window where the grab says
//! (`Event::WindowDragged`), and the clients under the pointer hear nothing meanwhile.

use super::feed;
use crate::state::State;
use casement_core::geometry::LogicalPoint;
use casement_core::{Event, GrabSpec};
use casement_wayland::seat::Drag;

/// Whether the grab took the pointer motion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Follow {
    /// A grab moved its window: the motion goes no further.
    Taken,
    /// No grab: the motion goes to the clients.
    Free,
}

/// A grab of `spec` begins with the pointer where it is.
pub(crate) fn start(state: &mut State, spec: GrabSpec) {
    let at = state
        .wl
        .seat
        .seat
        .get_pointer()
        .map_or(LogicalPoint { x: 0, y: 0 }, |pointer| {
            let at = pointer.current_location();
            LogicalPoint {
                x: at.x.floor() as i32,
                y: at.y.floor() as i32,
            }
        });
    state.wl.seat.grab = Some(Drag::begin(spec, at));
}

/// The pointer is at `at`: the grabbed window follows.
pub(super) fn follow(state: &mut State, at: LogicalPoint) -> Follow {
    let Some(drag) = state.wl.seat.grab else {
        return Follow::Free;
    };
    let window = drag.window();
    let origin = drag.origin.or_else(|| {
        state
            .core
            .arrangement
            .windows
            .iter()
            .find(|target| target.window == window)
            .map(|target| target.rect)
    });
    let Some(origin) = origin else {
        // The window is gone: the grab ends.
        state.wl.seat.grab = None;
        return Follow::Free;
    };
    state.wl.seat.grab = Some(drag.from_rect(origin));
    let rect = drag.rect_at(origin, at);
    feed(state, vec![Event::WindowDragged { window, rect }]);
    Follow::Taken
}

/// A button was released: the grab ends.
pub(super) fn end(state: &mut State) {
    state.wl.seat.grab = None;
}
