//! The fake process table: `FakeSpawner` records what would have started (A5), so a test
//! asserts the satellite's command line and the programs bindings launch. A clone shares the
//! record.

use casement::spawn::{EnvVar, SpawnError, SpawnSpec, Spawned, Spawner};
use casement_core::Pid;
use casement_core::input::Argv;
use std::sync::{Arc, Mutex, PoisonError};

/// One program the fake "started".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartedProgram {
    /// The program and its arguments.
    pub argv: Argv,
    /// The variables it was given.
    pub env: Vec<EnvVar>,
    /// How many descriptors it inherited.
    pub passed_fds: usize,
}

/// A spawner that starts nothing.
#[derive(Debug, Clone, Default)]
pub struct FakeSpawner {
    started: Arc<Mutex<Vec<StartedProgram>>>,
}

impl FakeSpawner {
    /// A spawner with nothing started.
    pub fn new() -> FakeSpawner {
        FakeSpawner::default()
    }

    /// The programs started so far, in order.
    pub fn started(&self) -> Vec<StartedProgram> {
        self.started
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

/// The first pid the fake hands out.
const FIRST_PID: u32 = 40_000;

impl Spawner for FakeSpawner {
    fn spawn(&mut self, spec: SpawnSpec) -> Result<Spawned, SpawnError> {
        if spec.argv.0.is_empty() {
            return Err(SpawnError::Empty);
        }
        let mut started = self.started.lock().unwrap_or_else(PoisonError::into_inner);
        let pid = FIRST_PID.saturating_add(u32::try_from(started.len()).unwrap_or(u32::MAX));
        started.push(StartedProgram {
            argv: spec.argv,
            env: spec.env,
            passed_fds: spec.pass.len(),
        });
        Ok(Spawned::new(Pid(pid)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn programs_are_recorded_and_an_empty_command_is_refused() {
        let mut fake = FakeSpawner::new();
        let reader = fake.clone();
        let argv = Argv(vec!["foot".to_owned()]);
        let started = fake.spawn(SpawnSpec::new(argv.clone())).expect("started");
        assert_eq!(started.pid, Pid(FIRST_PID));
        assert_eq!(reader.started().first().map(|p| p.argv.clone()), Some(argv));
        assert!(matches!(
            fake.spawn(SpawnSpec::new(Argv(Vec::new()))),
            Err(SpawnError::Empty)
        ));
    }
}
