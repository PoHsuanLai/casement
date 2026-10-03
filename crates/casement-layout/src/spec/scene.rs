//! The whole scene.

use super::OutputSpec;

/// Every output to arrange.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneSpec {
    /// The outputs, in the model's order.
    pub outputs: Vec<OutputSpec>,
}
