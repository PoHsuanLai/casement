//! Publishing the desktop's state to protocol clients, and keeping the Spaces across a restart
//! (P1-Z; filled by P1-M2; P2-Q3 adds the placement mark).

use super::log;
use crate::space_store::save_spaces;
use crate::state::State;
use casement_core::window::{SpacePlace, Window};
use casement_core::{Model, OutputKey, WindowId};
use casement_wayland::{capture_source, quire_window, wlr_toplevel, workspace};
use smithay::output::Output;

/// Writes the Spaces file through `state.seams.space_store` (`Model::space_file`). A failure is
/// logged and degrades.
pub(super) fn persist_spaces(state: &mut State) {
    let file = state.core.model.space_file();
    if let Err(error) = save_spaces(state.seams.space_store.as_mut(), &file) {
        tracing::warn!(%error, "the Spaces file could not be written");
    }
}

/// Sends `ext_workspace` clients the Spaces as the model now has them.
pub(super) fn publish_workspaces(state: &mut State) {
    log(workspace::apply_spaces(
        &mut state.wl.workspace,
        state.core.model.spaces(),
        &state.wl.output.outputs,
    ));
}

/// Sends `zwlr_foreign_toplevel_management` clients `window` as it now is.
pub(super) fn publish_toplevel(state: &mut State, window: WindowId) {
    let Some(held) = state.core.model.windows().get(&window) else {
        return;
    };
    let on: Vec<Output> = output_of(&state.core.model, held)
        .and_then(|key| state.wl.output.outputs.get(&key).cloned())
        .into_iter()
        .collect();
    log(wlr_toplevel::apply_toplevel(
        &mut state.wl.wlr_toplevel,
        held,
        &on,
    ));
    // The same trigger tells an extended window where it now sits (a no-op when the mark did
    // not change).
    log(quire_window::apply_placement(
        &mut state.wl.quire_window,
        window,
        held.state.mark(),
    ));
}

/// The output `window` is on: its Space's, or its home when it is on every Space.
fn output_of(model: &Model, window: &Window) -> Option<OutputKey> {
    match &window.place {
        SpacePlace::On(id) => model.spaces().get(id).map(|space| space.output.clone()),
        SpacePlace::AllSpaces => window.home.clone(),
    }
}

/// `window` closed: the capture sessions on it end (each client is told `stopped`).
pub(super) fn end_window_captures(state: &mut State, window: WindowId) {
    let gone = capture_source::apply_window_gone(&mut state.wl.capture_source.book, window);
    state.wl.capture_source.stop(&gone);
    state.wl.image_capture.copy.cleanup();
}
