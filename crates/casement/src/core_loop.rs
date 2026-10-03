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

/// The frame the scene is at now (what a golden of motion reads).
pub fn sample_scene(state: &State) -> casement_motion::FrameScene {
    let _ = state;
    todo!("P0-B-loop: core_loop::sample_scene")
}

/// Reports the outputs the backend plugged at start (once, after the clock and display are
/// attached, so start-up and hotplug are one path).
pub fn announce_outputs(state: &mut State) {
    let _ = state;
    todo!("P0-B-loop: core_loop::announce_outputs")
}

/// Plugs a virtual output on the headless backend and reports it to the model; its name.
pub fn plug_output(
    state: &mut State,
    spec: casement_backend::VirtualOutputSpec,
) -> casement_core::OutputName {
    let _ = (state, spec);
    todo!("P0-B-loop: core_loop::plug_output")
}

/// Unplugs `name` on the headless backend and reports it to the model.
pub fn unplug_output(state: &mut State, name: &casement_core::OutputName) {
    let _ = (state, name);
    todo!("P0-B-loop: core_loop::unplug_output")
}

/// What happens once per loop turn: queued input is delivered and a dead locker noticed; how
/// many things moved.
pub fn turn(state: &mut State) -> usize {
    let _ = state;
    todo!("P0-B-loop: core_loop::turn")
}
