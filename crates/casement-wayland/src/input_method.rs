//! `zwp_input_method_manager_v2`: the input method (fcitx5) and its popups. Hidden from
//! sandboxed clients (I7).

use crate::states::GlobalFilter;
use casement_core::Event;
use casement_core::policy::GlobalKind;
use smithay::input::SeatHandler;
use smithay::reexports::wayland_protocols_misc::zwp_input_method_v2::server::{
    zwp_input_method_manager_v2::ZwpInputMethodManagerV2, zwp_input_method_v2::ZwpInputMethodV2,
};
use smithay::reexports::wayland_server::{Dispatch, DisplayHandle, GlobalDispatch};
use smithay::wayland::GlobalData;
use smithay::wayland::input_method::{
    InputMethodManagerGlobalData, InputMethodManagerState, InputMethodUserData, PopupSurface,
};

/// `zwp_input_method_manager_v2`.
#[derive(Debug)]
pub struct InputMethodPart {
    /// smithay's state.
    pub state: InputMethodManagerState,
}

impl InputMethodPart {
    /// Creates the global (version 1), filtered by the I7 table.
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> InputMethodPart
    where
        D: GlobalDispatch<ZwpInputMethodManagerV2, InputMethodManagerGlobalData>
            + Dispatch<ZwpInputMethodManagerV2, GlobalData>
            + Dispatch<ZwpInputMethodV2, InputMethodUserData<D>>
            + SeatHandler
            + 'static,
    {
        InputMethodPart {
            state: InputMethodManagerState::new::<D, _>(
                display,
                role_filter.for_global(GlobalKind::InputMethod),
            ),
        }
    }
}

/// The input method committed text or preedit for the focused field.
pub fn on_im_commit(part: &mut InputMethodPart) -> Vec<Event> {
    let _ = part;
    todo!("P0-H7: input_method::on_im_commit")
}

/// The input method opened, moved or dismissed a candidate popup.
pub fn on_im_popup(part: &mut InputMethodPart, popup: &PopupSurface) -> Vec<Event> {
    let _ = (part, popup);
    todo!("P0-H7: input_method::on_im_popup")
}
