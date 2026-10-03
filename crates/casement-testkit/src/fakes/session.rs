//! The fake session: smithay's `Session` over memfd fixtures. Any open of a path that is not a
//! fixture is `DeniedOpen`, which fails the test with exit 5 (a jail breach, §6.1).

use casement_backend::SessionFault;
use rustix::fs::{MemfdFlags, OFlags, SeekFrom};
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

/// Whether the session holds the seat (another VT has it when not).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Activity {
    /// This session is on the screen.
    #[default]
    Active,
    /// Another VT has the seat: no device may be opened.
    Inactive,
}

/// The seat the fake session reports.
const SEAT: &str = "seat0";

/// A session that opens only its fixtures.
#[derive(Debug, Default)]
pub struct FakeSession {
    fixtures: BTreeMap<PathBuf, SessionFixture>,
    activity: Activity,
    /// The VT the session was last asked to change to.
    switched_to: Option<i32>,
}

impl FakeSession {
    /// A session with `fixtures`.
    pub fn with(fixtures: Vec<SessionFixture>) -> FakeSession {
        FakeSession {
            fixtures: fixtures.into_iter().map(|f| (f.path.clone(), f)).collect(),
            ..FakeSession::default()
        }
    }

    /// The VT the session was last asked to switch to, if any.
    pub fn switched_to(&self) -> Option<i32> {
        self.switched_to
    }

    /// The seat is given back (the VT the session lives on is current again).
    pub fn resume(&mut self) {
        self.activity = Activity::Active;
    }
}

/// A memfd holding `bytes`, read position at the start.
fn memfd_of(path: &Path, bytes: &[u8]) -> Result<OwnedFd, SessionFault> {
    let denied = |_| SessionFault::DeniedOpen(path.display().to_string());
    let fd = rustix::fs::memfd_create("casement-session-fixture", MemfdFlags::CLOEXEC)
        .map_err(denied)?;
    let mut rest = bytes;
    while !rest.is_empty() {
        let written = rustix::io::write(&fd, rest).map_err(denied)?;
        rest = rest.get(written..).unwrap_or_default();
    }
    rustix::fs::seek(&fd, SeekFrom::Start(0)).map_err(denied)?;
    Ok(fd)
}

impl Session for FakeSession {
    type Error = SessionFault;

    fn open(&mut self, path: &Path, flags: OFlags) -> Result<OwnedFd, SessionFault> {
        let _ = flags;
        match (self.activity, self.fixtures.get(path)) {
            (Activity::Inactive, _) => Err(SessionFault::Inactive),
            (Activity::Active, Some(fixture)) => memfd_of(path, &fixture.bytes),
            (Activity::Active, None) => Err(SessionFault::DeniedOpen(path.display().to_string())),
        }
    }

    fn close(&mut self, fd: OwnedFd) -> Result<(), SessionFault> {
        drop(fd);
        Ok(())
    }

    fn change_vt(&mut self, vt: i32) -> Result<(), SessionFault> {
        self.switched_to = Some(vt);
        self.activity = Activity::Inactive;
        Ok(())
    }

    fn is_active(&self) -> bool {
        self.activity == Activity::Active
    }

    fn seat(&self) -> String {
        SEAT.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read_all(fd: &OwnedFd) -> Vec<u8> {
        let mut out = Vec::new();
        let mut chunk = [0_u8; 64];
        loop {
            match rustix::io::read(fd, &mut chunk) {
                Ok(0) | Err(_) => return out,
                Ok(n) => out.extend_from_slice(chunk.get(..n).unwrap_or_default()),
            }
        }
    }

    fn session() -> FakeSession {
        FakeSession::with(vec![SessionFixture {
            path: PathBuf::from("/dev/dri/card0"),
            bytes: b"recorded bytes".to_vec(),
        }])
    }

    #[test]
    fn a_fixture_opens_as_a_memfd_with_its_bytes() {
        let mut session = session();
        let fd = session
            .open(Path::new("/dev/dri/card0"), OFlags::RDWR)
            .expect("fixture");
        assert_eq!(read_all(&fd), b"recorded bytes");
    }

    #[test]
    fn any_other_open_is_denied() {
        let mut session = session();
        for path in ["/dev/dri/card1", "/dev/input/event0", "/etc/passwd"] {
            assert_eq!(
                session.open(Path::new(path), OFlags::RDONLY).unwrap_err(),
                SessionFault::DeniedOpen(path.to_owned())
            );
        }
    }

    #[test]
    fn a_vt_switch_pauses_the_session_until_it_resumes() {
        let mut session = session();
        assert!(session.is_active());
        session.change_vt(3).expect("switch");
        assert!(!session.is_active());
        assert_eq!(session.switched_to(), Some(3));
        assert_eq!(
            session
                .open(Path::new("/dev/dri/card0"), OFlags::RDWR)
                .unwrap_err(),
            SessionFault::Inactive
        );
        session.resume();
        assert!(session.is_active());
        assert_eq!(session.seat(), "seat0");
    }
}
