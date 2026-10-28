//! Starting programs (A5, P1-Z): the `Spawner` seam. A binding's program (`Effect::Spawn`) and
//! xwayland-satellite go through it, so no code in the compositor calls `Command` itself and a
//! test asserts what would have started (`casement-testkit`'s `FakeSpawner`).
//!
//! A program inherits the descriptors in [`SpawnSpec::pass`] (the listening sockets
//! xwayland-satellite takes): the real spawner clears their close-on-exec flag, and the
//! program's arguments name their numbers.
//!
//! A program a binding starts runs in a scope of its own in the person's service manager
//! ([`Home::OwnScope`]), so restarting the compositor does not end the terminals, builds and
//! editors it started. Nothing a program starts inherits the compositor's service-manager
//! variables ([`SERVICE_VARS`]). The real spawner is also the one owner of its children: it
//! reaps them and says which ended ([`Spawner::exited`]), so the X11 bridge learns of its exit.
//!
//! ```no_run
//! use casement::spawn::{ProcessSpawner, SpawnSpec, Spawner};
//! use casement_core::input::Argv;
//!
//! let mut spawner = ProcessSpawner::new(None);
//! let spec = SpawnSpec::new(Argv(vec!["foot".to_owned()]));
//! let started = spawner.spawn(spec).expect("started");
//! let _ = started.pid;
//! ```

mod process;
mod scope;

pub use process::ProcessSpawner;
pub use scope::{scope_argv, unit_for};

use casement_core::Pid;
use casement_core::input::Argv;
use casement_core::time::Mono;
use std::os::fd::OwnedFd;

/// The variables a service manager sets for the service itself. A program casement starts is
/// not that service, and one that kept them would talk to the manager as the compositor
/// (`NOTIFY_SOCKET`, `WATCHDOG_*`) or take its descriptors (`LISTEN_*`).
pub const SERVICE_VARS: [&str; 6] = [
    "NOTIFY_SOCKET",
    "WATCHDOG_USEC",
    "WATCHDOG_PID",
    "LISTEN_PID",
    "LISTEN_FDS",
    "LISTEN_FDNAMES",
];

/// One environment variable for the program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvVar {
    /// The name.
    pub name: String,
    /// The value.
    pub value: String,
}

/// Where a program runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum Home {
    /// With the compositor: it ends when the compositor does (the X11 bridge).
    #[default]
    WithCompositor,
    /// In a scope of its own in the person's service manager, when there is one: the compositor
    /// restarting leaves it running. A plain child when there is no manager.
    OwnScope,
}

/// The variables every program casement starts gets, set once the compositor knows them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChildEnv(Vec<EnvVar>);

impl ChildEnv {
    /// The same with `name` set to `value`.
    #[must_use]
    pub fn with(self, name: &str, value: &str) -> ChildEnv {
        let mut vars = self.0;
        vars.retain(|var| var.name != name);
        vars.push(EnvVar {
            name: name.to_owned(),
            value: value.to_owned(),
        });
        ChildEnv(vars)
    }

    /// `spec` with these variables added; one the spec already sets keeps its own value.
    pub fn apply(&self, spec: SpawnSpec) -> SpawnSpec {
        self.0.iter().fold(spec, |spec, var| {
            match spec.env.iter().any(|set| set.name == var.name) {
                true => spec,
                false => spec.with_env(&var.name, &var.value),
            }
        })
    }
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
    /// Where it runs.
    pub home: Home,
}

impl SpawnSpec {
    /// A program with no extra environment and no descriptors, running with the compositor.
    pub fn new(argv: Argv) -> SpawnSpec {
        SpawnSpec {
            argv,
            env: Vec::new(),
            pass: Vec::new(),
            home: Home::WithCompositor,
        }
    }

    /// The same, running in a scope of its own.
    #[must_use]
    pub fn in_own_scope(self) -> SpawnSpec {
        SpawnSpec {
            home: Home::OwnScope,
            ..self
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

    /// The programs that ended since the last call, as of `now` (the session clock's). The one
    /// that started a program is the one that reaps it, so this is the only place an exit is
    /// seen. A spawner that starts nothing real reports none.
    fn exited(&mut self, _now: Mono) -> Vec<Pid> {
        Vec::new()
    }
}
