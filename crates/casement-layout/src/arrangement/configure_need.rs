//! Whether a window's client must be asked for a new size.

use casement_core::geometry::LogicalSize;

/// Whether layout needs a configure for a window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigureNeed {
    /// Its committed size fits.
    Keep,
    /// Ask it for this size.
    Resize(LogicalSize),
}
