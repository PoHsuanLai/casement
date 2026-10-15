//! What the model tells the shell's protocol layer (the `Effect::Shell` of this crate).

use crate::geometry::{LogicalPoint, LogicalRect};
use crate::ids::{LayerKey, WindowId};
use crate::window::{MenuItem, SplitSide, enabled_mask};
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
    /// A window went full screen to share the screen in a Split View: the shell shows the other
    /// windows of the desktop to pick from and answers with
    /// [`ShellRequest::SplitPick`](super::ShellRequest::SplitPick) or
    /// [`ShellRequest::SplitCancel`](super::ShellRequest::SplitCancel).
    SplitChoose(SplitChoices),
    /// The choosing for this first window is over (a pick, a cancel, or the window left): the
    /// shell takes its picker away.
    SplitChoiceEnd(WindowId),
}

/// The other half of a Split View is being chosen.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SplitChoices {
    /// The window already full screen on its half.
    pub first: WindowId,
    /// The half it took.
    pub side: SplitSide,
    /// The windows that can fill the other half, most recently used first.
    pub candidates: Vec<WindowId>,
}

impl SplitChoices {
    /// The choice for the half beside `first`, offering `candidates` in order.
    ///
    /// ```
    /// use casement_core::WindowId;
    /// use casement_core::shell::SplitChoices;
    /// use casement_core::window::SplitSide;
    ///
    /// let choices = SplitChoices::new(WindowId(1), SplitSide::Left, vec![WindowId(2)]);
    /// assert_eq!(choices.candidates, [WindowId(2)]);
    /// ```
    pub fn new(first: WindowId, side: SplitSide, candidates: Vec<WindowId>) -> Self {
        Self {
            first,
            side,
            candidates,
        }
    }
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
