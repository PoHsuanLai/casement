//! The Wayland display on the event loop, without `unsafe` (SP3): the display is shared with
//! the source that dispatches its clients, and the loop's data is the `State` itself.

use crate::state::{StartError, State};
use smithay::reexports::calloop::generic::Generic;
use smithay::reexports::calloop::{Interest, LoopHandle, Mode, PostAction};
use smithay::reexports::wayland_server::Display;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The display, shared by its owner (the binary's main, the test kit's `Fixture`) and its
/// event source; and how many client messages the source has dispatched (a fixture reads it
/// to know a round moved).
#[derive(Debug, Clone)]
pub struct SharedDisplay(pub Rc<RefCell<Display<State>>>, pub Rc<Cell<u64>>);

impl SharedDisplay {
    /// How many client messages have been dispatched so far.
    pub fn dispatched(&self) -> u64 {
        self.1.get()
    }
}

/// Inserts a source over a duplicate of `display`'s poll fd into `event_loop` that dispatches
/// client requests into the `State`; returns the shared display (its owner flushes clients
/// after each loop turn).
pub fn attach_display(
    mut display: Display<State>,
    event_loop: &LoopHandle<'static, State>,
) -> Result<SharedDisplay, StartError> {
    let fd = display
        .backend()
        .poll_fd()
        .try_clone_to_owned()
        .map_err(|error| StartError::Loop(format!("the display's poll fd: {error}")))?;
    let shared = SharedDisplay(Rc::new(RefCell::new(display)), Rc::new(Cell::new(0)));
    let (display, dispatched) = (shared.0.clone(), shared.1.clone());
    event_loop
        .insert_source(
            Generic::new(fd, Interest::READ, Mode::Level),
            move |_, _, state: &mut State| {
                match display.borrow_mut().dispatch_clients(state) {
                    Ok(n) => dispatched.set(dispatched.get().saturating_add(n as u64)),
                    Err(error) => tracing::warn!(%error, "dispatching clients failed"),
                }
                Ok(PostAction::Continue)
            },
        )
        .map_err(|error| StartError::Loop(error.to_string()))?;
    Ok(shared)
}

/// Sends every client its queued events.
pub fn flush(display: &SharedDisplay) {
    if let Err(error) = display.0.borrow_mut().flush_clients() {
        tracing::warn!(%error, "flushing clients failed");
    }
}
