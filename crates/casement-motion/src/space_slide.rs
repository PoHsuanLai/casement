//! A one-dimensional leg: a Space strip's offset or an overview's progress.

use casement_core::time::Mono;
use ds_motion::spring::Leg;

/// One leg and when it began.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SlideLeg {
    /// The leg (logical pixels for a strip, 0 to 1 for an overview).
    pub(crate) leg: Leg,
    /// When it began.
    pub(crate) began: Mono,
}
