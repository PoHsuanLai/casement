//! `zwp_keyboard_shortcuts_inhibit_manager_v1` (P0-H4).

use crate::state::State;
use smithay::wayland::keyboard_shortcuts_inhibit::{
    KeyboardShortcutsInhibitHandler, KeyboardShortcutsInhibitState,
};

impl KeyboardShortcutsInhibitHandler for State {
    fn keyboard_shortcuts_inhibit_state(&mut self) -> &mut KeyboardShortcutsInhibitState {
        &mut self.wl.shortcuts_inhibit.state
    }
}
