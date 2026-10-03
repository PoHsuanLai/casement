//! A window's legs: its rectangle's four edges' worth (origin and size) and its opacity.

use casement_core::time::Mono;
use ds_motion::spring::Leg;

/// The legs that move one window, all begun at `began`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct WindowLegs {
    /// The left edge, in logical pixels.
    pub(crate) x: Leg,
    /// The top edge.
    pub(crate) y: Leg,
    /// The width.
    pub(crate) w: Leg,
    /// The height.
    pub(crate) h: Leg,
    /// The opacity, 0 to 1.
    pub(crate) opacity: Leg,
    /// When these legs began.
    pub(crate) began: Mono,
}
