//! The origins a test may mint (C19): the mintable subset of `casement_core::input::Origin`.
//! Agent input enters only through a lease's EIS socket (P4); a feed only through palmrest's
//! socket (P3).

use casement_core::input::{DeviceClass, Origin, PortalSessionId};

/// Where a test's input claims to come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TestOrigin {
    /// A kernel device of this class.
    Physical(DeviceClass),
    /// A RemoteDesktop session.
    Remote,
    /// Plain test input.
    Test,
}

impl TestOrigin {
    /// The model's origin (a remote origin is the test kit's portal session 0).
    pub fn origin(self) -> Origin {
        match self {
            TestOrigin::Physical(class) => Origin::Physical(class),
            TestOrigin::Remote => Origin::Remote(PortalSessionId(0)),
            TestOrigin::Test => Origin::Test,
        }
    }
}
