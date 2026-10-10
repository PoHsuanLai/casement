//! Requests from workspace clients (`ext_workspace_manager_v1`) on the Spaces. P1-M2 fills it.

use super::Model;
use super::moved::Moved;
use crate::space::WorkspaceRequest;

impl Model {
    /// A workspace client asked something of the Spaces: show, hide, create, remove or move
    /// one. `Effect::PersistSpaces` and `Effect::PublishWorkspaces` follow every change.
    pub(super) fn workspace_request(self, request: WorkspaceRequest) -> Moved {
        let _ = (self, request);
        todo!("P1-M2: Model::workspace_request")
    }
}
