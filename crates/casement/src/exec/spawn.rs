//! A binding's program (A5, P1-Z; filled by P1-I1): `Effect::Spawn` goes to the injected
//! `Spawner` seam, with the environment X11 clients need (`DISPLAY`) when satellite runs.

use crate::state::State;
use casement_core::input::Argv;

/// Starts `argv` through `state.seams.spawner`. A failure is logged and degrades.
pub(super) fn spawn_program(state: &mut State, argv: Argv) {
    let _ = (state, argv);
    todo!("P1-I1: exec::spawn::spawn_program")
}
