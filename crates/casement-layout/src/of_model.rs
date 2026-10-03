//! The spec of a model: the one place layout reads casement-core's model.

use crate::SceneSpec;
use casement_core::Model;

/// What `model` asks layout to arrange: every plugged output with its usable area (after
/// layer exclusive zones), scale, mode and Spaces, each Space with its windows in stacking
/// order.
pub fn spec_of(model: &Model) -> SceneSpec {
    let _ = model;
    todo!("P0-F1-layout: outputs, usable areas, Spaces and windows from the model")
}
