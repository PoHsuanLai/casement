//! A binding's program (A5, P1-Z; filled by P1-I1): `Effect::Spawn` goes to the injected
//! `Spawner` seam, with the environment X11 clients need (`DISPLAY`) when satellite runs.

use crate::spawn::SpawnSpec;
use crate::state::State;
use casement_core::input::Argv;

/// Starts `argv` through `state.seams.spawner`. A failure is logged and degrades.
pub(super) fn spawn_program(state: &mut State, argv: Argv) {
    let spec = SpawnSpec::new(argv);
    // DISPLAY from the moment casement listens for X11 clients, not only once satellite runs.
    let spec = state
        .xwayland
        .env()
        .into_iter()
        .fold(spec, |spec, var| spec.with_env(&var.name, &var.value));
    match state.seams.spawner.spawn(spec) {
        Ok(started) => tracing::debug!(pid = started.pid.0, "a binding started a program"),
        Err(error) => tracing::warn!(%error, "a binding's program did not start"),
    }
}
