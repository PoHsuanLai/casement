//! `ext_workspace_manager_v1` (P1-M2): callbacks that feed `Event::Workspace`.

use crate::core_loop;
use crate::state::State;
use casement_core::Event;
use casement_wayland::workspace::{WorkspaceHandler, WorkspacePart};

impl WorkspaceHandler for State {
    fn workspace_part(&mut self) -> &mut WorkspacePart {
        &mut self.wl.workspace
    }

    fn workspace_events(&mut self, events: Vec<Event>) {
        core_loop::feed(self, events);
    }
}
