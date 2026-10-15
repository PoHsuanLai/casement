//! The winit backend's turn: what the window reported since the last turn, told to the model by
//! the paths the tty backend uses (a connector change, input events), and whether the user
//! closed the window.

use super::tty::{desktop_extent, with_scroll_factor};
use super::{connector, feed};
use crate::state::State;
use casement_backend::BackendKind;
use casement_backend::input::{FedInput, PollContext};
use casement_core::Event;
use std::ops::ControlFlow;

/// Drains the window: the output's changes first (so an event after a resize is placed on the
/// new extent next turn), then the input. `Break` once the window was closed; a backend that is
/// not winit reports nothing.
pub(crate) fn drain(state: &mut State) -> ControlFlow<()> {
    let context = PollContext::new(state.clock.now(), desktop_extent(state));
    let BackendKind::Winit(window) = &mut state.backend else {
        return ControlFlow::Continue(());
    };
    let turn = window.dispatch(&context);
    for change in turn.connectors {
        connector(state, change);
    }
    for item in turn.fed {
        if let FedInput::Event(event) = item {
            let event = with_scroll_factor(state, event);
            feed(state, vec![Event::Input(event)]);
        }
    }
    turn.flow
}
