//! `Effect::Shell` (P2-Z; filled by P2-Q2): records or sends what concerns the shell.

use super::log;
use crate::state::State;
use casement_core::shell::ShellNote;
use casement_wayland::quire::shell;

/// Carries out one note for the shell. Also the way the edge reports the real modifier keys
/// (`ShellNote::Modifiers`), which no model transition makes.
pub(super) fn note(state: &mut State, note: ShellNote) {
    log(shell::apply_note(&mut state.wl.quire_shell, &note));
}
