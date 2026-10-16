//! `zwp_input_method_manager_v2`: the input method (fcitx5) and its popups. Hidden from
//! sandboxed clients (I7).

use crate::states::GlobalFilter;
use casement_core::Event;
use casement_core::policy::GlobalKind;
use smithay::input::SeatHandler;
use smithay::reexports::wayland_protocols_misc::zwp_input_method_v2::server::{
    zwp_input_method_manager_v2::ZwpInputMethodManagerV2, zwp_input_method_v2::ZwpInputMethodV2,
};
use smithay::reexports::wayland_server::{Dispatch, DisplayHandle, GlobalDispatch, Resource};
use smithay::wayland::input_method::{
    InputMethodManagerGlobalData, InputMethodManagerState, InputMethodManagerUserData,
    InputMethodUserData, PopupSurface,
};
use std::collections::BTreeMap;

/// `zwp_input_method_manager_v2`.
#[derive(Debug)]
pub struct InputMethodPart {
    /// smithay's state.
    pub state: InputMethodManagerState,
    /// The candidate popups that are open on a field, for the scene to draw above their
    /// parent.
    pub popups: BTreeMap<PopupKey, PopupSurface>,
}

/// An input-popup role object, by its protocol id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PopupKey(pub u32);

impl PopupKey {
    /// The key of `popup`.
    pub fn of(popup: &PopupSurface) -> PopupKey {
        PopupKey(popup.surface_role.id().protocol_id())
    }
}

impl InputMethodPart {
    /// Creates the global (version 1), filtered by the I7 table.
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> InputMethodPart
    where
        D: GlobalDispatch<ZwpInputMethodManagerV2, InputMethodManagerGlobalData>
            + Dispatch<ZwpInputMethodManagerV2, InputMethodManagerUserData>
            + Dispatch<ZwpInputMethodV2, InputMethodUserData<D>>
            + SeatHandler
            + 'static,
    {
        InputMethodPart {
            state: InputMethodManagerState::new::<D, _>(
                display,
                role_filter.for_global(GlobalKind::InputMethod),
            ),
            popups: BTreeMap::new(),
        }
    }
}

/// The input method committed text or preedit for the focused field. smithay relays
/// `commit_string`, `set_preedit_string` and `commit` between the input method and the field
/// itself and the model has no text state, so there is nothing to feed it.
pub fn on_im_commit(part: &mut InputMethodPart) -> Vec<Event> {
    let _ = part;
    Vec::new()
}

/// The input method opened or moved a candidate popup over its field's surface: the popup is
/// kept (replacing an older record of the same popup) while it is alive and has a parent.
pub fn on_im_popup(part: &mut InputMethodPart, popup: &PopupSurface) -> Vec<Event> {
    let key = PopupKey::of(popup);
    if popup.alive() && popup.get_parent().is_some() {
        part.popups.insert(key, popup.clone());
    } else {
        part.popups.remove(&key);
    }
    Vec::new()
}

/// The input method's candidate popup was dismissed (the field lost focus or the method
/// deactivated).
pub fn on_im_popup_dismissed(part: &mut InputMethodPart, popup: &PopupSurface) -> Vec<Event> {
    part.popups.remove(&PopupKey::of(popup));
    Vec::new()
}
