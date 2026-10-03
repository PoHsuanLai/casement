//! The core loop: `Model::step` → `Relayout` → arrange → retarget → frames.

use crate::state::State;
use casement_core::{Event, OutputKey};

/// Steps every event at the clock's now and carries out the effects ([`crate::exec::exec`]);
/// a `Relayout` arranges once per feed (not once per event), retargets motion and asks the
/// outputs for frames.
pub fn feed(state: &mut State, events: Vec<Event>) {
    let _ = (state, events);
    todo!("P0-B-loop: core_loop::feed")
}

/// One frame on `output`: sample motion at the clock's now, build the elements, render,
/// present, send frame callbacks and presentation feedback at the frame's instant.
pub fn frame(state: &mut State, output: &OutputKey) {
    let _ = (state, output);
    todo!("P0-B-loop: core_loop::frame")
}
