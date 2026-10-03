//! `zwp_keyboard_shortcuts_inhibit_manager_v1`: a focused surface (a VM, a remote desktop)
//! asks for every key, bindings included. The kill chord is never inhibited (P4).

use crate::states::GlobalFilter;
use casement_core::Event;
use smithay::reexports::wayland_protocols::wp::keyboard_shortcuts_inhibit::zv1::server::{
    zwp_keyboard_shortcuts_inhibit_manager_v1::ZwpKeyboardShortcutsInhibitManagerV1,
    zwp_keyboard_shortcuts_inhibitor_v1::ZwpKeyboardShortcutsInhibitorV1,
};
use smithay::reexports::wayland_server::{Dispatch, DisplayHandle, GlobalDispatch};
use smithay::wayland::GlobalData;
use smithay::wayland::keyboard_shortcuts_inhibit::{
    KeyboardShortcutsInhibitState, KeyboardShortcutsInhibitor, KeyboardShortcutsInhibitorUserData,
};

/// `zwp_keyboard_shortcuts_inhibit_manager_v1`.
#[derive(Debug)]
pub struct ShortcutsInhibitPart {
    /// smithay's state.
    pub state: KeyboardShortcutsInhibitState,
}

impl ShortcutsInhibitPart {
    /// Creates the global (version 1). Every connection sees it.
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> ShortcutsInhibitPart
    where
        D: GlobalDispatch<ZwpKeyboardShortcutsInhibitManagerV1, GlobalData>
            + Dispatch<ZwpKeyboardShortcutsInhibitManagerV1, GlobalData>
            + Dispatch<ZwpKeyboardShortcutsInhibitorV1, KeyboardShortcutsInhibitorUserData>
            + 'static,
    {
        let _ = role_filter;
        ShortcutsInhibitPart {
            state: KeyboardShortcutsInhibitState::new::<D>(display),
        }
    }
}

/// A surface asked to inhibit shortcuts (activated while it has keyboard focus).
pub fn on_inhibit(
    part: &mut ShortcutsInhibitPart,
    inhibitor: &KeyboardShortcutsInhibitor,
) -> Vec<Event> {
    let _ = (part, inhibitor);
    todo!("P0-H4: shortcuts_inhibit::on_inhibit")
}
