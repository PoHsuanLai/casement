//! The Wayland display on the event loop, without `unsafe` (SP3): the display is shared with
//! the source that dispatches its clients, and the loop's data is the `State` itself.

use crate::state::{StartError, State};
use smithay::reexports::calloop::LoopHandle;
use smithay::reexports::wayland_server::Display;
use std::cell::RefCell;
use std::rc::Rc;

/// The display, shared by its owner (the binary's main, the test kit's `Fixture`) and its
/// event source.
#[derive(Debug, Clone)]
pub struct SharedDisplay(pub Rc<RefCell<Display<State>>>);

/// Inserts a source over a duplicate of `display`'s poll fd into `event_loop` that dispatches
/// client requests into the `State`; returns the shared display (its owner flushes clients
/// after each loop turn).
pub fn attach_display(
    display: Display<State>,
    event_loop: &LoopHandle<'static, State>,
) -> Result<SharedDisplay, StartError> {
    let _ = (display, event_loop);
    todo!("P0-B-loop: attach_display")
}

/// Sends every client its queued events.
pub fn flush(display: &SharedDisplay) {
    let _ = display;
    todo!("P0-B-loop: display::flush")
}
