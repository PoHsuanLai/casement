//! The session seam's types.

use smithay::backend::session::{AsErrno, Session};

/// Who opens devices for casement.
pub enum SessionSeam {
    /// Nobody: the headless and winit backends open no device.
    None,
    /// A fake over fixtures (tests only): any open outside its fixtures is `DeniedOpen`.
    Fake(Box<dyn Session<Error = SessionFault>>),
}

impl std::fmt::Debug for SessionSeam {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionSeam::None => f.write_str("SessionSeam::None"),
            SessionSeam::Fake(session) => write!(f, "SessionSeam::Fake(seat {})", session.seat()),
        }
    }
}

/// Why a session refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum SessionFault {
    /// The open was refused (in a test: the jail is breached; the test fails with exit 5).
    #[error("denied open {0}")]
    DeniedOpen(String),
    /// The session is not active (another VT has it).
    #[error("inactive")]
    Inactive,
    /// The seat could not be opened or was lost (libseat, udev); the text is the adapter's.
    #[error("seat unavailable: {0}")]
    Unavailable(String),
}

impl AsErrno for SessionFault {
    fn as_errno(&self) -> Option<i32> {
        match self {
            SessionFault::DeniedOpen(_) => Some(rustix::io::Errno::ACCESS.raw_os_error()),
            SessionFault::Inactive | SessionFault::Unavailable(_) => None,
        }
    }
}
