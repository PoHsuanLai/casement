//! The fake session: smithay's `Session` over memfd fixtures. Any open of a path that is not a
//! fixture is `DeniedOpen`, which fails the test with exit 5 (a jail breach, §6.1).

use super::logind::manager::SleepPhase;
use casement_backend::SessionFault;
use casement_backend::tty::{DevicePath, SessionNote, TtySession, VtNumber};
use rustix::fs::{MemfdFlags, OFlags, SeekFrom};
use smithay::backend::session::Session;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::os::fd::OwnedFd;
use std::path::{Path, PathBuf};
use std::rc::Rc;

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
    /// The seat changes not yet handed out by `notes`.
    pending: Vec<SessionNote>,
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
        self.set(Activity::Active);
    }

    /// logind's `PrepareForSleep` as the session hears it: the seat is taken away when the
    /// machine is about to sleep and given back when it woke. (The fake logind server, P1-F2,
    /// drives this through the same two calls.)
    pub fn prepare_for_sleep(&mut self, phase: SleepPhase) {
        match phase {
            SleepPhase::Starting => self.set(Activity::Inactive),
            SleepPhase::Resumed => self.set(Activity::Active),
        }
    }

    /// Moves to `activity`, noting a change of hands (and nothing when it did not change).
    fn set(&mut self, activity: Activity) {
        if self.activity == activity {
            return;
        }
        self.activity = activity;
        self.pending.push(match activity {
            Activity::Active => SessionNote::Activated,
            Activity::Inactive => SessionNote::Paused,
        });
    }
}

/// A [`FakeSession`] that a backend and a test both hold: the backend takes one clone as its
/// `TtySession`, the test drives the seat through another.
///
/// ```
/// use casement_testkit::fakes::{FakeSession, SharedSession};
///
/// let session = SharedSession::new(FakeSession::default());
/// let for_the_backend = session.clone();
/// session.resume();
/// drop(for_the_backend);
/// ```
#[derive(Debug, Clone)]
pub struct SharedSession(Rc<RefCell<FakeSession>>);

impl SharedSession {
    /// Shares `session`.
    pub fn new(session: FakeSession) -> SharedSession {
        SharedSession(Rc::new(RefCell::new(session)))
    }

    /// The seat is given back.
    pub fn resume(&self) {
        self.0.borrow_mut().resume();
    }

    /// logind's `PrepareForSleep`.
    pub fn prepare_for_sleep(&self, phase: SleepPhase) {
        self.0.borrow_mut().prepare_for_sleep(phase);
    }

    /// The VT the session was last asked to switch to, if any.
    pub fn switched_to(&self) -> Option<i32> {
        self.0.borrow().switched_to()
    }
}

impl TtySession for SharedSession {
    fn open_device(&mut self, path: &DevicePath) -> Result<OwnedFd, SessionFault> {
        self.0.borrow_mut().open_device(path)
    }

    fn switch_vt(&mut self, vt: VtNumber) -> Result<(), SessionFault> {
        self.0.borrow_mut().switch_vt(vt)
    }

    fn notes(&mut self) -> Vec<SessionNote> {
        self.0.borrow_mut().notes()
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
        self.set(Activity::Inactive);
        Ok(())
    }

    fn is_active(&self) -> bool {
        self.activity == Activity::Active
    }

    fn seat(&self) -> String {
        SEAT.to_owned()
    }
}

impl TtySession for FakeSession {
    fn open_device(&mut self, path: &DevicePath) -> Result<OwnedFd, SessionFault> {
        Session::open(self, path.as_path(), OFlags::RDWR)
    }

    fn switch_vt(&mut self, vt: VtNumber) -> Result<(), SessionFault> {
        let vt = i32::try_from(vt.0).map_err(|_| SessionFault::Inactive)?;
        Session::change_vt(self, vt)
    }

    fn notes(&mut self) -> Vec<SessionNote> {
        std::mem::take(&mut self.pending)
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

    #[test]
    fn notes_are_handed_out_once_and_only_for_changes() {
        let mut session = session();
        session.resume();
        assert_eq!(session.notes(), []);
        session.prepare_for_sleep(SleepPhase::Starting);
        session.prepare_for_sleep(SleepPhase::Starting);
        assert_eq!(session.notes(), [SessionNote::Paused]);
        assert_eq!(session.notes(), []);
        session.prepare_for_sleep(SleepPhase::Resumed);
        assert_eq!(session.notes(), [SessionNote::Activated]);
    }
}
