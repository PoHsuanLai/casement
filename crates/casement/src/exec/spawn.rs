//! A binding's program (A5, P1-Z; filled by P1-I1): `Effect::Spawn` goes to the injected
//! `Spawner` seam, in a scope of its own, with the environment its windows need: `DISPLAY` for
//! X11 clients when satellite runs and `WAYLAND_DISPLAY` for this compositor's socket.

use crate::spawn::{ChildEnv, EnvVar, Home, SpawnSpec};
use crate::state::State;
use casement_core::input::Argv;

/// What to start for a binding's `argv`: in a scope of its own, so a compositor restart leaves
/// the program running; `x11` (DISPLAY, from the moment casement listens for X11 clients, not
/// only once satellite runs) and then `child_env` (WAYLAND_DISPLAY: the socket this run bound,
/// which is not `wayland-0` when that one is taken) as its environment.
fn spec_for(argv: Argv, x11: Vec<EnvVar>, child_env: &ChildEnv) -> SpawnSpec {
    let spec = x11
        .into_iter()
        .fold(SpawnSpec::new(argv).in_own_scope(), |spec, var| {
            spec.with_env(&var.name, &var.value)
        });
    child_env.apply(spec)
}

/// Starts `argv` through `state.seams.spawner`. A failure is logged and degrades.
pub(super) fn spawn_program(state: &mut State, argv: Argv) {
    let spec = spec_for(argv, state.xwayland.env(), &state.seams.child_env);
    match state.seams.spawner.spawn(spec) {
        Ok(started) => tracing::debug!(pid = started.pid.0, "a binding started a program"),
        Err(error) => tracing::warn!(%error, "a binding's program did not start"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_binding_program_gets_its_own_scope_and_both_display_names() {
        let x11 = vec![EnvVar {
            name: "DISPLAY".to_owned(),
            value: ":3".to_owned(),
        }];
        let child_env = ChildEnv::default().with("WAYLAND_DISPLAY", "wayland-1");
        let spec = spec_for(Argv(vec!["foot".to_owned()]), x11, &child_env);
        assert_eq!(spec.home, Home::OwnScope);
        let env: Vec<(&str, &str)> = spec
            .env
            .iter()
            .map(|var| (var.name.as_str(), var.value.as_str()))
            .collect();
        assert_eq!(env, [("DISPLAY", ":3"), ("WAYLAND_DISPLAY", "wayland-1")]);
    }
}
