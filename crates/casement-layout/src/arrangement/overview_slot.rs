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
}

/// What an overview slot shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SlotKind {
    /// A shown window, spread or exposed.
    Live,
    /// A minimised window's thumbnail (App Exposé's lower row, a fixed 16:9).
    Minimised,
}
