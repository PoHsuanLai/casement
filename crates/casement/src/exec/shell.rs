//! `Effect::Shell` (P2-Z; filled by P2-Q2): records or sends what concerns the shell.

use super::log;
use crate::state::State;
use casement_core::overview::Overview;
use casement_core::shell::ShellNote;
use casement_protocols::server::quire_shell_v1::quire_shell_v1::Shown;
use casement_wayland::quire::{shell, shell_screen, shell_space};

/// Carries out one note for the shell. Also the way the edge reports the real modifier keys
/// (`ShellNote::Modifiers`), which no model transition makes.
pub(super) fn note(state: &mut State, note: ShellNote) {
    match &note {
        // Mission Control is global (as the Mac's with a Space row on every display): every
        // output is up while it is not closed. The outputs are told first, then the state.
        ShellNote::Overview(overview) => {
            let shown = match overview {
                Overview::Closed => Shown::Hidden,
                _ => Shown::Shown,
            };
            shell_space::send_overview_outputs(
                &state.wl.quire_shell,
                &state.wl.output.outputs,
                shown,
            );
        }
        ShellNote::DropTarget { output, space } => {
            shell_space::send_drop_target(
                &state.wl.quire_shell,
                &state.wl.output.outputs,
                output,
                space.as_ref(),
            );
            return;
        }
        ShellNote::DropTargetNew { output, over } => {
            let over = match over {
                casement_protocols::vocab::Shown::Shown => Shown::Shown,
                _ => Shown::Hidden,
            };
            shell_space::send_drop_target_new(
                &state.wl.quire_shell,
                &state.wl.output.outputs,
                output,
                over,
            );
            return;
        }
        ShellNote::WindowDragBegin { output, window } => {
            shell_space::send_window_drag_begin(
                &state.wl.quire_shell,
                &state.wl.output.outputs,
                output,
                *window,
            );
            return;
        }
        ShellNote::WindowDragEnd {
            output,
            window,
            outcome,
        } => {
            shell_space::send_window_drag_end(
                &state.wl.quire_shell,
                &state.wl.output.outputs,
                output,
                *window,
                *outcome,
            );
            return;
        }
        ShellNote::Reveal(output, edge, shown) => {
            shell_screen::send_reveal(
                &state.wl.quire_shell,
                &state.wl.output.outputs,
                output,
                *edge,
                *shown,
            );
            return;
        }
        _ => {}
    }
    log(shell::apply_note(&mut state.wl.quire_shell, &note));
}
