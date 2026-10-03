//! The layout seam.

use crate::{Arrangement, LayoutKnobs, SceneSpec};

/// Where every window, Space and overview slot belongs, given what the model holds. Pure and
/// time-free: the same spec and knobs give the same arrangement. Implementations:
/// [`crate::RustScene`] (production), `FixedScene` (tests), `CssScene` (the excluded parity
/// oracle).
pub trait SceneLayout {
    /// The arrangement for `spec` under `knobs`.
    fn arrange(&self, spec: &SceneSpec, knobs: &LayoutKnobs) -> Arrangement;
}
