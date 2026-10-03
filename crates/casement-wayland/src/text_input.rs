//! `zwp_text_input_manager_v3`: text fields. smithay ties text-input focus to keyboard focus;
//! the first key after `enable` must reach the input method, not the document (SP1 A5).

use crate::states::{GlobalFilter, WaylandFault};
use casement_core::Event;
use smithay::input::{Seat, SeatHandler};
use smithay::reexports::wayland_protocols::wp::text_input::zv3::server::{
    zwp_text_input_manager_v3::ZwpTextInputManagerV3, zwp_text_input_v3::ZwpTextInputV3,
};
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::{Dispatch, DisplayHandle, GlobalDispatch};
use smithay::wayland::GlobalData;
use smithay::wayland::text_input::{TextInputManagerState, TextInputUserData};

/// `zwp_text_input_manager_v3`.
#[derive(Debug)]
pub struct TextInputPart {
    /// smithay's state.
    pub state: TextInputManagerState,
}

impl TextInputPart {
    /// Creates the global (version 1). Every connection sees it.
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> TextInputPart
    where
        D: GlobalDispatch<ZwpTextInputManagerV3, GlobalData>
            + Dispatch<ZwpTextInputManagerV3, GlobalData>
            + Dispatch<ZwpTextInputV3, TextInputUserData>
            + 'static,
    {
        let _ = role_filter;
        TextInputPart {
            state: TextInputManagerState::new::<D>(display),
        }
    }
}

/// A focused text field enabled text input.
pub fn on_enable(part: &mut TextInputPart, surface: &WlSurface) -> Vec<Event> {
    let _ = (part, surface);
    todo!("P0-H7: text_input::on_enable")
}

/// A text field committed its state (surrounding text, content type, cursor rectangle).
pub fn on_commit_state(part: &mut TextInputPart, surface: &WlSurface) -> Vec<Event> {
    let _ = (part, surface);
    todo!("P0-H7: text_input::on_commit_state")
}

/// Moves text-input focus with keyboard focus.
pub fn apply_text_focus<D: SeatHandler + 'static>(
    seat: &Seat<D>,
    focus: Option<&WlSurface>,
) -> Result<(), WaylandFault> {
    let _ = (seat, focus);
    todo!("P0-H7: text_input::apply_text_focus")
}

/// Sends the input method's preedit to the focused field.
pub fn apply_preedit<D: SeatHandler + 'static>(
    seat: &Seat<D>,
    text: &str,
    cursor: (i32, i32),
) -> Result<(), WaylandFault> {
    let _ = (seat, text, cursor);
    todo!("P0-H7: text_input::apply_preedit")
}

/// Commits text to the focused field.
pub fn apply_commit_string<D: SeatHandler + 'static>(
    seat: &Seat<D>,
    text: &str,
) -> Result<(), WaylandFault> {
    let _ = (seat, text);
    todo!("P0-H7: text_input::apply_commit_string")
}
