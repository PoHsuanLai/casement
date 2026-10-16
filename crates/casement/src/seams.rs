//! What the composition root is injected with (A1, A4, A5): the file, bus and process seams.
//!
//! `State::new` builds the real ones from the start configuration ([`Seams::real`]); the test
//! kit's `Fixture` builds the fakes and calls `State::with_seams`. Nothing in the loop opens a
//! file, calls a bus or starts a process any other way.

use crate::notify::{NoNotify, Notify};
use crate::space_store::{EphemeralSpaceStore, FileSpaceStore, SpaceStore, StateHome};
use crate::space_sync::SpaceSync;
use crate::spawn::{ChildEnv, ProcessSpawner, Spawner};
use casement_backend::tty::TtySeams;
use casement_ipc::shell_calls::{ShellCalls, ZbusShellCalls};

/// The injected seams.
#[derive(Debug)]
#[non_exhaustive]
pub struct Seams {
    /// Where the Spaces file is stored.
    pub space_store: Box<dyn SpaceStore>,
    /// How a binding calls sill.
    pub shell_calls: Box<dyn ShellCalls>,
    /// How a program is started.
    pub spawner: Box<dyn Spawner>,
    /// The variables every program a binding starts gets (the Wayland socket's name, known once
    /// `run` has bound it).
    pub child_env: ChildEnv,
    /// The Spaces registered with the account service (none until `run` is given a bus).
    pub space_sync: SpaceSync,
    /// How the service manager is told casement is ready (nothing until `run` is given a socket).
    pub notify: Box<dyn Notify>,
    /// The tty backend's seams, when they are injected (a test); none builds the real ones.
    pub tty: Option<TtySeams>,
}

impl Seams {
    /// These seams.
    pub fn new(
        space_store: Box<dyn SpaceStore>,
        shell_calls: Box<dyn ShellCalls>,
        spawner: Box<dyn Spawner>,
    ) -> Seams {
        Seams {
            space_store,
            shell_calls,
            spawner,
            child_env: ChildEnv::default(),
            space_sync: SpaceSync::none(),
            notify: Box::new(NoNotify),
            tty: None,
        }
    }

    /// The same with Spaces registered through `space_sync`.
    #[must_use]
    pub fn with_space_sync(self, space_sync: SpaceSync) -> Seams {
        Seams { space_sync, ..self }
    }

    /// The same with the service manager told through `notify`.
    #[must_use]
    pub fn with_notify(self, notify: Box<dyn Notify>) -> Seams {
        Seams { notify, ..self }
    }

    /// The same with the tty backend's seams injected (the FakeKms suite).
    #[must_use]
    pub fn with_tty(self, tty: TtySeams) -> Seams {
        Seams {
            tty: Some(tty),
            ..self
        }
    }

    /// The real seams: the Spaces file in `home` (none kept for an ephemeral run), the session
    /// bus on first use, child processes.
    pub fn real(home: &StateHome) -> Seams {
        let space_store: Box<dyn SpaceStore> = match home {
            StateHome::At(dirs) => Box::new(FileSpaceStore::new(dirs)),
            StateHome::Ephemeral => Box::new(EphemeralSpaceStore),
        };
        Seams::new(
            space_store,
            Box::new(ZbusShellCalls::lazy()),
            Box::new(ProcessSpawner::default()),
        )
    }
}
