//! What layout could not honour.

use casement_core::geometry::LogicalSize;
use casement_core::{OutputKey, WindowId};

/// A rule layout had to break, reported rather than hidden.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutReport {
    /// A window's minimum size could not be met (it does not fit the area, or a Split pair's
    /// minimums do not fit together).
    MinUnmet {
        /// The window.
        window: WindowId,
        /// Its minimum.
        min: LogicalSize,
        /// What it got.
        got: LogicalSize,
    },
    /// Mission Control could not separate every window at the spread floor; overlaps remain.
    SpreadOverflow(OutputKey),
    /// App Exposé's rows are closer than the gap.
    RowsCrowded(OutputKey),
}
