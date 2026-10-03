//! The scale a display gets the first time it is seen (12 §4: the policy moves from sill's
//! display service into the compositor, so a 4K output is right before any client draws).

use super::OutputFacts;
use ds_core::geometry::scale::Scale;
use serde::{Deserialize, Serialize};

/// How a display's first scale is chosen (`outputs.first_sight_scale`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum ScalePolicy {
    /// Aim at the Mac's 27-inch density (109 points per inch) and prefer a whole scale where
    /// the ideal is at least 1.5 and the display stays at least 1280 points wide, as macOS does
    /// for a sharp external display; otherwise the nearest quarter step. With no usable
    /// physical size: 2 for a panel at least 3840 pixels wide, else 1.
    WholeLikeMac,
    /// Always this scale.
    #[serde(with = "crate::geometry::scale_serde")]
    Fixed(Scale),
}

/// The scale for a display seen for the first time.
pub fn first_sight_scale(facts: &OutputFacts, policy: &ScalePolicy) -> Scale {
    let _ = (facts, policy);
    todo!("P0-F1-output: density over the preferred mode, rounded per the policy")
}
