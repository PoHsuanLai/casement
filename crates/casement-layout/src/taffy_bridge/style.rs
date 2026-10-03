//! Taffy styles from logical sizes.

use super::round::to_float;
use casement_core::geometry::LogicalSize;
use taffy::{Dimension, Size};

/// A fixed Taffy size of `size` logical pixels.
pub(crate) fn fixed_size(size: LogicalSize) -> Size<Dimension> {
    Size {
        width: Dimension::length(to_float(size.w)),
        height: Dimension::length(to_float(size.h)),
    }
}
