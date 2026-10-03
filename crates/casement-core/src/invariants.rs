//! What must hold after every step ([`crate::Model::check_invariants`]).

use crate::app::AppKey;
use crate::ids::{OutputKey, WindowId};
use porter_core::SpaceId;

/// The first invariant found broken.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvariantBreak {
    /// The key window is not a mapped window.
    KeyWindowNotMapped(WindowId),
    /// A window is on a Space that does not exist.
    WindowWithoutSpace(WindowId),
    /// A Space belongs to an output that is not plugged.
    SpaceOnMissingOutput(SpaceId),
    /// A window's app is not in the app table.
    AppWithoutKey(WindowId),
    /// The active app is hidden.
    FocusOnHiddenApp(AppKey),
    /// Two Spaces of one output share a position.
    DuplicateSpaceIndex(OutputKey),
}
