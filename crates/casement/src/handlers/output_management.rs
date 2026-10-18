//! `zwlr_output_manager_v1` (P1-M1): `casement_wayland::output_management` serves the
//! protocol with its own `GlobalDispatch2`/`Dispatch2`; this file gives it the root's part,
//! the model's outputs and the loop (an applied configuration is one `Event::OutputRequested`
//! per head), and holds [`sync_outputs`], which the core loop runs after every batch of events
//! so that the heads, the power controls and the gamma controls follow the model.

use crate::state::State;
use casement_core::OutputBook;
use casement_wayland::output_management::{OutputManagementHandler, OutputManagementPart};
use casement_wayland::{gamma, output_management, output_power};

impl OutputManagementHandler for State {
    fn output_management_part(&mut self) -> &mut OutputManagementPart {
        &mut self.wl.output_management
    }

    fn output_management_outputs(&self) -> &OutputBook {
        self.core.model.outputs()
    }
}

/// Brings every client of an output-facing protocol up to date with the model: new, changed
/// and gone heads (with a new serial and `done`), the power the controls report, and the
/// gamma controls of outputs that left. Cheap when nothing changed.
pub(crate) fn sync_outputs(state: &mut State) {
    let book = state.core.model.outputs();
    if let Err(fault) = output_management::apply_heads::<State>(
        &mut state.wl.output_management,
        &state.display,
        book,
    ) {
        tracing::warn!(%fault, "the output heads did not follow the model");
    }
    output_power::follow(&mut state.wl.output_power, book);
    gamma::follow(&mut state.wl.gamma, book);
}
