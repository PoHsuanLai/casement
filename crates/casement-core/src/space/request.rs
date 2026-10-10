//! What a workspace client (`ext_workspace_manager_v1`) asks of the Spaces.

use crate::ids::OutputKey;
use porter_core::SpaceId;

/// One request on a Space or on an output's group of Spaces.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum WorkspaceRequest {
    /// Show this Space on its output.
    Activate(SpaceId),
    /// Stop showing this Space (the output shows another).
    Deactivate(SpaceId),
    /// Remove this Space, its windows moving to a neighbour.
    Remove(SpaceId),
    /// Make a new Space in this output's row.
    Create {
        /// The output.
        output: OutputKey,
        /// The name the client suggested.
        name: String,
    },
    /// Move this Space to another output's row.
    Assign {
        /// The Space.
        space: SpaceId,
        /// The output.
        output: OutputKey,
    },
}
