//! A window's size limits (`xdg_toplevel.set_min_size`/`set_max_size`): the model holds them
//! and layout reads them (C51).

use crate::geometry::LogicalSize;

/// How small and how large a window may be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SizeLimits {
    /// The smallest size (zero on a side means none).
    pub min: LogicalSize,
    /// The largest size.
    pub max: Option<LogicalSize>,
}
