//! One window's slot in an overview.

use casement_core::WindowId;
use casement_core::geometry::LogicalRect;

/// Where a window is drawn while an overview is open (its client keeps its size).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverviewSlot {
    /// The window.
    pub window: WindowId,
    /// Live or a minimised thumbnail.
    pub kind: SlotKind,
    /// Its slot, in global logical coordinates.
    pub rect: LogicalRect,
    /// Where the app's icon sits at rest: centred under the slot, overlapping its bottom edge,
    /// in global logical coordinates. The renderer draws it with the slot as the slot arrives
    /// (the layout holds no pixels; the icon is the window's app's).
    pub icon: LogicalRect,
}

/// What an overview slot shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SlotKind {
    /// A shown window, spread or exposed.
    Live,
    /// A minimised window's thumbnail (App Exposé's lower row, a fixed 16:9).
    Minimised,
}
