//! Starting programs (A5, P1-Z): the `Spawner` seam. A binding's program (`Effect::Spawn`) and
//! xwayland-satellite go through it, so no code in the compositor calls `Command` itself and a
//! test asserts what would have started (`casement-testkit`'s `FakeSpawner`).
//!
//! A program inherits the descriptors in [`SpawnSpec::pass`] (the listening sockets
//! xwayland-satellite takes): the real spawner clears their close-on-exec flag, and the
//! program's arguments name their numbers.
//!
//! ```no_run
//! use casement::spawn::{ProcessSpawner, SpawnSpec, Spawner};
//! use casement_core::input::Argv;
//!
//! let mut spawner = ProcessSpawner;
//! let spec = SpawnSpec::new(Argv(vec!["foot".to_owned()]));
//! let started = spawner.spawn(spec).expect("started");
//! let _ = started.pid;
//! ```

use casement_core::Pid;
use casement_core::input::Argv;
use std::os::fd::OwnedFd;

/// One environment variable for the program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvVar {
    /// The name.
    pub name: String,
    /// The value.
    pub value: String,
}

/// What to start.
#[derive(Debug)]
#[non_exhaustive]
pub struct SpawnSpec {
    /// The program and its arguments.
    pub argv: Argv,
    /// Variables added to the program's environment.
    pub env: Vec<EnvVar>,
    /// Descriptors the program inherits, by their own numbers.
    pub pass: Vec<OwnedFd>,
}

impl SpawnSpec {
    /// A program with no extra environment and no descriptors.
    pub fn new(argv: Argv) -> SpawnSpec {
        SpawnSpec {
            argv,
            env: Vec::new(),
            pass: Vec::new(),
        }
    }

    /// The same, with this variable set.
    #[must_use]
    pub fn with_env(self, name: &str, value: &str) -> SpawnSpec {
        let mut env = self.env;
        env.push(EnvVar {
            name: name.to_owned(),
            value: value.to_owned(),
        });
        SpawnSpec { env, ..self }
    }

    /// The same, passing `fd` to the program.
    #[must_use]
    pub fn with_fd(self, fd: OwnedFd) -> SpawnSpec {
        let mut pass = self.pass;
        pass.push(fd);
        SpawnSpec { pass, ..self }
    }
}

/// A program that started.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Spawned {
    /// Its process id.
    pub pid: Pid,
}

impl Spawned {
    /// A started program.
    pub fn new(pid: Pid) -> Spawned {
        Spawned { pid }
    }
}

/// Why a program did not start.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SpawnError {
    /// The argument list was empty.
    #[error("nothing to start: the argument list is empty")]
    Empty,
    /// The program could not be started.
    #[error("{program}: {source}")]
    Start {
        /// The program as named.
        program: String,
        /// Why it failed.
        #[source]
        source: std::io::Error,
    },
}

/// Starts programs.
pub trait Spawner: std::fmt::Debug {
    /// Starts the program `spec` names.
    fn spawn(&mut self, spec: SpawnSpec) -> Result<Spawned, SpawnError>;
}

/// The real spawner: a child process.
#[derive(Debug, Default)]
pub struct ProcessSpawner;

impl Spawner for ProcessSpawner {
    fn spawn(&mut self, spec: SpawnSpec) -> Result<Spawned, SpawnError> {
        let _ = spec;
        todo!("P1-I1: ProcessSpawner::spawn")
    }
}
