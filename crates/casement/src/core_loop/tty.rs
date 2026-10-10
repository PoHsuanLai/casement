//! The tty backend's turn: what the seat and the devices reported since the last turn, told to
//! the model by the same paths the headless backend uses.

use super::connector;
use crate::exec;
use crate::state::State;
use casement_backend::BackendKind;
use casement_backend::tty::{SessionNote, TtyNote};
use casement_core::Effect;

/// Drains the tty backend: connector events go down the connector path (plug, change,
/// unplug); a pause needs nothing here (the backend presents no frame while the seat is lent);
/// a return re-applies every output's configuration, which re-modesets the displays. How many
/// things the backend reported. A backend that is not tty reports nothing.
pub(super) fn drain(state: &mut State) -> usize {
    let BackendKind::Tty(tty) = &mut state.backend else {
        return 0;
    };
    let notes = tty.dispatch();
    let count = notes.len();
    for note in notes {
        match note {
            TtyNote::Connector(event) => connector(state, event),
            TtyNote::Session(SessionNote::Activated) => remodeset(state),
            _ => {}
        }
    }
    count
}

/// Puts the model's configuration of every output back on the backend.
fn remodeset(state: &mut State) {
    let effects: Vec<Effect> = state
        .core
        .model
        .outputs()
        .0
        .iter()
        .map(|(output, plugged)| Effect::ApplyOutput {
            output: output.clone(),
            config: plugged.config,
        })
        .collect();
    exec::exec(state, effects);
}
