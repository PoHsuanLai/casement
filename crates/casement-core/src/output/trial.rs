//! Who asked for a display change, and how long the person has to keep one a program made.
//!
//! A change to the displays by anything but the shell is a trial: it is applied at once, the
//! person is asked "Keep these display settings?" and the previous setup comes back unless
//! they keep it (`model/display_trial.rs`). The shell's own changes are the person's already.

use crate::policy::ConnectionRole;
use std::time::Duration;

/// How long the person has to keep a display change a program made: long enough to read the
/// sheet and press a key, short enough that someone staring at a black screen (a bad mode can
/// blank it) is back in a quarter of a minute.
pub const DISPLAY_CONFIRM_WITHIN: Duration = Duration::from_secs(15);

/// Who asked for a change to an output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Requester {
    /// The shell's own connection: the person is at the desk, so the change is never asked
    /// about, and it settles a trial in flight as kept.
    Shell,
    /// Any other connection: the change is a trial the person keeps or reverts.
    Program,
    /// The compositor putting back what a program had changed (a display a program darkened
    /// is lit again when its control goes): never asked, and a trial in flight is left alone.
    Restore,
}

impl Requester {
    /// Who a request is from, by the socket its connection came through.
    pub fn of(role: &ConnectionRole) -> Requester {
        match role {
            ConnectionRole::Shell => Requester::Shell,
            ConnectionRole::Agent
            | ConnectionRole::Public
            | ConnectionRole::Portal
            | ConnectionRole::Sandboxed(_) => Requester::Program,
        }
    }
}
