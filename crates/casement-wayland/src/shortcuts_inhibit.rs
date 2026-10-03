//! `zwp_keyboard_shortcuts_inhibit_manager_v1`: a focused surface (a VM, a remote desktop)
//! asks for every key, bindings included. The kill chord is never inhibited (P4).

use crate::states::GlobalFilter;
use casement_core::Event;
use smithay::input::{Seat, SeatHandler};
use smithay::reexports::wayland_protocols::wp::keyboard_shortcuts_inhibit::zv1::server::{
    zwp_keyboard_shortcuts_inhibit_manager_v1::ZwpKeyboardShortcutsInhibitManagerV1,
    zwp_keyboard_shortcuts_inhibitor_v1::ZwpKeyboardShortcutsInhibitorV1,
};
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::{Dispatch, DisplayHandle, GlobalDispatch};
use smithay::wayland::GlobalData;
use smithay::wayland::keyboard_shortcuts_inhibit::{
    KeyboardShortcutsInhibitState, KeyboardShortcutsInhibitor, KeyboardShortcutsInhibitorSeat,
    KeyboardShortcutsInhibitorUserData,
};

/// `zwp_keyboard_shortcuts_inhibit_manager_v1`.
#[derive(Debug)]
pub struct ShortcutsInhibitPart {
    /// smithay's state.
    pub state: KeyboardShortcutsInhibitState,
    /// The inhibitors clients created, in creation order.
    pub inhibitors: Vec<KeyboardShortcutsInhibitor>,
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
            inhibitors: Vec::new(),
        }
    }
}

/// Whether the bindings are being taken, for the key path to ask before it resolves a chord.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Bindings {
    /// Chords resolve as usual.
    Active,
    /// The focused client asked for every key: nothing resolves, every key is forwarded.
    Suspended,
}

/// A surface asked to inhibit shortcuts. It is remembered and activated while its surface has
/// keyboard focus ([`sync_focus`]); nothing is reported to the model.
pub fn on_inhibit(
    part: &mut ShortcutsInhibitPart,
    inhibitor: &KeyboardShortcutsInhibitor,
) -> Vec<Event> {
    part.inhibitors.push(inhibitor.clone());
    Vec::new()
}

/// An inhibitor was destroyed.
pub fn on_inhibitor_destroyed(
    part: &mut ShortcutsInhibitPart,
    inhibitor: &KeyboardShortcutsInhibitor,
) -> Vec<Event> {
    part.inhibitors
        .retain(|held| held.wl_surface() != inhibitor.wl_surface());
    Vec::new()
}

/// Keyboard focus moved to `focused`: its inhibitor (if it asked for one) becomes active and
/// every other one inactive.
pub fn sync_focus(part: &ShortcutsInhibitPart, focused: Option<&WlSurface>) {
    for inhibitor in &part.inhibitors {
        if Some(inhibitor.wl_surface()) == focused {
            inhibitor.activate();
        } else {
            inhibitor.inactivate();
        }
    }
}

/// Whether `seat`'s bindings are suspended by an active inhibitor.
pub fn bindings<D: SeatHandler + 'static>(seat: &Seat<D>) -> Bindings {
    if seat.keyboard_shortcuts_inhibited() {
        Bindings::Suspended
    } else {
        Bindings::Active
    }
}
