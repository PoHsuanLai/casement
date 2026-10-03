//! Placing outputs in the global logical space.

use super::{OutputConfig, OutputFacts, SavedOutputs, ScalePolicy};
use crate::ids::OutputKey;

/// The configuration of every present output: a saved display keeps its saved configuration;
/// a new one gets its preferred mode, its first-sight scale under `policy`, no transform,
/// power on, and a place to the right of everything already placed, top-aligned. In
/// `present`'s order.
pub fn place_outputs(
    present: &[OutputFacts],
    saved: &SavedOutputs,
    policy: &ScalePolicy,
) -> Vec<(OutputKey, OutputConfig)> {
    let _ = (present, saved, policy);
    todo!("P0-F1-output: saved first, new ones in a row to the right")
}
