//! What the composition root is injected with (A1, A4, A5): the file, bus and process seams.
//!
//! `State::new` builds the real ones from the start configuration ([`Seams::real`]); the test
//! kit's `Fixture` builds the fakes and calls `State::with_seams`. Nothing in the loop opens a
//! file, calls a bus or starts a process any other way.

use crate::space_store::{EphemeralSpaceStore, FileSpaceStore, SpaceStore, StateHome};
use crate::spawn::{ProcessSpawner, Spawner};
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
            tty: None,
        }
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
            Box::new(ProcessSpawner),
        )
    }
}
