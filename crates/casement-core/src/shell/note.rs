//! What the model tells the shell's protocol layer (the `Effect::Shell` of this crate).

use crate::geometry::{LogicalPoint, LogicalRect};
use crate::ids::{LayerKey, WindowId};
use crate::window::{MenuItem, enabled_mask};
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
    /// Show the window menu: the shell draws it and answers with
    /// [`ShellRequest::WindowMenuPick`](super::ShellRequest::WindowMenuPick).
    WindowMenu(WindowMenuShow),
}

/// The window menu to show: for which window, where, and which entries can be picked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowMenuShow {
    /// The window the menu is for.
    pub window: WindowId,
    /// Where it opens, in global logical pixels.
    pub at: LogicalPoint,
    /// Every entry in menu order, each enabled or dimmed.
    pub items: Vec<MenuItem>,
}

impl WindowMenuShow {
    /// The enabled entries as the wire's bit set.
    ///
    /// ```
    /// use casement_core::WindowId;
    /// use casement_core::geometry::LogicalPoint;
    /// use casement_core::shell::WindowMenuShow;
    /// use casement_core::window::{MenuEntry, MenuItem, Offer};
    ///
    /// let show = WindowMenuShow {
    ///     window: WindowId(1),
    ///     at: LogicalPoint { x: 0, y: 0 },
    ///     items: vec![MenuItem { entry: MenuEntry::Fill, offer: Offer::Enabled }],
    /// };
    /// assert_eq!(show.enabled(), 1 << MenuEntry::Fill.code());
    /// ```
    pub fn enabled(&self) -> u32 {
        enabled_mask(&self.items)
    }
}
