//! `ext_workspace_manager_v1` (P1-M2): callbacks that feed `Event::Workspace`.

use crate::state::State;
use casement_wayland::workspace::{WorkspaceHandler, WorkspacePart};

impl WorkspaceHandler for State {
    fn workspace_part(&mut self) -> &mut WorkspacePart {
        &mut self.wl.workspace
    }
}
