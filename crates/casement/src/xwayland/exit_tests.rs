//! Satellite's exit: seen through the spawner that started it, never by waiting on its pid.

use super::*;
use crate::spawn::ProcessSpawner;
use std::time::Duration;

/// A spawner that "starts" every program as this pid.
#[derive(Debug)]
struct Fixed(u32);

impl Spawner for Fixed {
    fn spawn(&mut self, _spec: SpawnSpec) -> Result<Spawned, SpawnError> {
        Ok(Spawned::new(Pid(self.0)))
    }
}

fn running_with(spawner: &mut dyn Spawner, binary: &str, dir: &std::path::Path) -> XwaylandState {
    let sockets = Listening::bind(&X11Dir(dir.to_owned())).expect("a free display");
    let started = Xwayland::start(
        &XwaylandConfig::satellite(binary),
        spawner,
        sockets,
        &WaylandSocket("wayland-1".to_owned()),
    );
    match started {
        Ok(running) => XwaylandState::Running(running),
        Err((error, _)) => panic!("satellite did not start: {error}"),
    }
}

#[test]
fn only_the_exit_of_the_satellite_itself_returns_the_sockets_to_listening() {
    let dir = tempfile::tempdir().expect("a scratch folder");
    let mut state = running_with(&mut Fixed(77), "xwayland-satellite", dir.path());
    state.reap(&[]);
    state.reap(&[Pid(5)]);
    assert!(matches!(state, XwaylandState::Running(_)));
    state.reap(&[Pid(5), Pid(77)]);
    assert!(matches!(state, XwaylandState::Listening(_)));
}

#[test]
fn a_real_satellite_that_exits_is_noticed_and_can_be_started_again() {
    let dir = tempfile::tempdir().expect("a scratch folder");
    // `true` stands in for a satellite that exits at once, whatever it is given.
    let mut spawner = ProcessSpawner::new(None);
    let mut state = running_with(&mut spawner, "true", dir.path());
    let back = (0..400).any(|turn| {
        std::thread::sleep(Duration::from_millis(10));
        state.reap(&spawner.exited(casement_core::time::Mono(Duration::from_secs(turn))));
        matches!(state, XwaylandState::Listening(_))
    });
    assert!(back, "the exit was never noticed");
}
