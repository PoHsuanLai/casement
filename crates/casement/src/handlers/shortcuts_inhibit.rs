//! `zwp_keyboard_shortcuts_inhibit_manager_v1` (P0-H4).

use crate::core_loop;
use crate::state::State;
use casement_wayland::shortcuts_inhibit;
use smithay::wayland::keyboard_shortcuts_inhibit::{
    KeyboardShortcutsInhibitHandler, KeyboardShortcutsInhibitState, KeyboardShortcutsInhibitor,
};

impl KeyboardShortcutsInhibitHandler for State {
    fn keyboard_shortcuts_inhibit_state(&mut self) -> &mut KeyboardShortcutsInhibitState {
        &mut self.wl.shortcuts_inhibit.state
    }

    fn new_inhibitor(&mut self, inhibitor: KeyboardShortcutsInhibitor) {
        let events = shortcuts_inhibit::on_inhibit(&mut self.wl.shortcuts_inhibit, &inhibitor);
        // An inhibitor on the surface that already holds the keyboard is active at once.
        let focused = self.wl.seat.keyboard.current_focus();
        shortcuts_inhibit::sync_focus(&self.wl.shortcuts_inhibit, focused.as_ref());
        core_loop::feed(self, events);
    }

    fn inhibitor_destroyed(&mut self, inhibitor: KeyboardShortcutsInhibitor) {
        let events =
            shortcuts_inhibit::on_inhibitor_destroyed(&mut self.wl.shortcuts_inhibit, &inhibitor);
        core_loop::feed(self, events);
    }
}
