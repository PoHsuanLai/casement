//! The fakes: what stands in for the session, udev, logind and palmrest inside the jail.

mod logind;
mod palmrest;
mod session;
mod udev;

pub use logind::{FakeLogind, LogindCall, LogindError};
pub use palmrest::FakePalmrest;
pub use session::{FakeSession, SessionFixture};
pub use udev::{FakeUdev, UdevEvent};
