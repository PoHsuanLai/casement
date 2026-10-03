//! The fake session: smithay's `Session` over memfd fixtures. Any open of a path that is not a
//! fixture is `DeniedOpen`, which fails the test with exit 5 (a jail breach, §6.1).

use casement_backend::SessionFault;
use rustix::fs::OFlags;
use smithay::backend::session::Session;
use std::collections::BTreeMap;
use std::os::fd::OwnedFd;
use std::path::{Path, PathBuf};

/// One device the fake session can open: its bytes, served from a memfd.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionFixture {
    /// The device path it answers (`/dev/dri/card0` in a FakeKms fixture).
    pub path: PathBuf,
    /// The bytes the memfd holds.
    pub bytes: Vec<u8>,
}

/// A session that opens only its fixtures.
#[derive(Debug, Default)]
pub struct FakeSession {
    #[expect(dead_code, reason = "P0-B-headless: the fixtures the session serves")]
    fixtures: BTreeMap<PathBuf, SessionFixture>,
}

impl FakeSession {
    /// A session with `fixtures`.
    pub fn with(fixtures: Vec<SessionFixture>) -> FakeSession {
        FakeSession {
            fixtures: fixtures.into_iter().map(|f| (f.path.clone(), f)).collect(),
        }
    }
}

impl Session for FakeSession {
    type Error = SessionFault;

    fn open(&mut self, path: &Path, flags: OFlags) -> Result<OwnedFd, SessionFault> {
        let _ = (path, flags);
        todo!("P0-B-headless: FakeSession::open")
    }

    fn close(&mut self, fd: OwnedFd) -> Result<(), SessionFault> {
        let _ = fd;
        todo!("P0-B-headless: FakeSession::close")
    }

    fn change_vt(&mut self, vt: i32) -> Result<(), SessionFault> {
        let _ = vt;
        todo!("P0-B-headless: FakeSession::change_vt")
    }

    fn is_active(&self) -> bool {
        todo!("P0-B-headless: FakeSession::is_active")
    }

    fn seat(&self) -> String {
        todo!("P0-B-headless: FakeSession::seat")
    }
}
