//! `org.freedesktop.login1.Seat` (P1-F2): `SwitchTo`, recorded.

use super::log::Recorder;
use super::{LogindCall, SEAT_ID, SESSION_ID, SESSION_PATH, object_path};
use zbus::fdo;
use zbus::zvariant::OwnedObjectPath;

/// The seat object. Served as a `#[zbus::interface]` by [`FakeLogind`](super::FakeLogind).
#[derive(Debug)]
pub struct Seat {
    log: Recorder,
}

impl Seat {
    pub(super) fn new(log: Recorder) -> Seat {
        Seat { log }
    }
}

#[zbus::interface(name = "org.freedesktop.login1.Seat")]
impl Seat {
    /// `SwitchTo(vt)`, recorded.
    fn switch_to(&self, vt: u32) {
        self.log.record(LogindCall::SwitchTo(vt));
    }

    #[zbus(property)]
    fn id(&self) -> String {
        SEAT_ID.to_owned()
    }

    #[zbus(property)]
    fn active_session(&self) -> fdo::Result<(String, OwnedObjectPath)> {
        Ok((SESSION_ID.to_owned(), object_path(SESSION_PATH)?))
    }

    #[zbus(property)]
    fn can_graphical(&self) -> bool {
        true
    }
}
