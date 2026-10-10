//! Publishing the desktop's state to protocol clients, and keeping the Spaces across a restart
//! (P1-Z; filled by P1-M2).

use crate::state::State;
use casement_core::WindowId;

/// Writes the Spaces file through `state.seams.space_store` (`Model::space_file`). A failure is
/// logged and degrades.
pub(super) fn persist_spaces(state: &mut State) {
    let _ = state;
    todo!("P1-M2: exec::publish::persist_spaces")
}

/// Sends `ext_workspace` clients the Spaces as the model now has them.
pub(super) fn publish_workspaces(state: &mut State) {
    let _ = state;
    todo!("P1-M2: exec::publish::publish_workspaces")
}

/// Sends `zwlr_foreign_toplevel_management` clients `window` as it now is.
pub(super) fn publish_toplevel(state: &mut State, window: WindowId) {
    let _ = (state, window);
    todo!("P1-M2: exec::publish::publish_toplevel")
}
