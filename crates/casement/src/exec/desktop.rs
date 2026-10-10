//! `Effect::Desktop` (P2-Q1): tells `quire_desktop_v1` clients what changed.

use super::log;
use crate::state::State;
use casement_core::desktop::DesktopNote;
use casement_wayland::quire::desktop::{self, DesktopView};

/// Sends every bound desktop manager the object `note` names as it now is.
pub(super) fn publish(state: &mut State, note: DesktopNote) {
    let view = DesktopView {
        model: &state.core.model,
        outputs: &state.wl.output.outputs,
        rects: state
            .core
            .arrangement
            .windows
            .iter()
            .map(|target| (target.window, target.rect))
            .collect(),
    };
    log(desktop::apply_note(
        &mut state.wl.quire_desktop,
        &view,
        &note,
    ));
}
