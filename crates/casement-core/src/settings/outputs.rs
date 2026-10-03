//! Output settings (`outputs.*`).

use crate::output::ScalePolicy;

/// `outputs.*`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutputSettings {
    /// `outputs.first_sight_scale`.
    pub first_sight: ScalePolicy,
}

impl OutputSettings {
    /// The Mac's whole-scale policy.
    pub(crate) fn defaults() -> OutputSettings {
        OutputSettings {
            first_sight: ScalePolicy::WholeLikeMac,
        }
    }
}
