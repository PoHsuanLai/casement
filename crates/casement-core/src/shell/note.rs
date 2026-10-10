//! What the model tells the shell's protocol layer (the `Effect::Shell` of this crate).

use crate::geometry::LogicalRect;
use crate::ids::{LayerKey, WindowId};
use casement_protocols::vocab::{Corner, MacMods, ScreenEdge, ShellRole, Shown};

/// Something the shell's protocol layer records or sends.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ShellNote {
    /// A layer surface took a role: the protocol layer records which surface holds it, so
    /// the events addressed to a role (modifiers) find it.
    Role(LayerKey, ShellRole),
    /// `window` now minimises into this rectangle (global logical pixels); the protocol layer
    /// has nothing to send until P3's dock bounce, so it only keeps the table.
    MinimiseTarget(WindowId, LogicalRect),
    /// The modifier keys held changed (`quire_shell_v1.modifiers`, to the role surfaces that
    /// ask for hot corners and the like).
    Modifiers(MacMods),
    /// The pointer entered a hot corner (P3).
    HotCorner(Corner),
    /// An edge reveal (the dock or the bar over a full-screen window) opened or closed (P3).
    Reveal(ScreenEdge, Shown),
}
