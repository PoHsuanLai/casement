//! `zwp_virtual_keyboard_manager_v1`: keyboards a client creates (on-screen keyboards, input
//! tools). Their keys are `Origin::Virtual` (I8: never physical) and count as idle activity.
//! smithay hands virtual keys straight to the focused client; casement must route them
//! through the model instead (FINDINGS). Hidden from sandboxed clients (I7).

use crate::states::GlobalFilter;
use casement_core::Event;
use casement_core::input::{Edge, KeyCode};
use casement_core::policy::GlobalKind;
use casement_core::time::Mono;
use smithay::input::SeatHandler;
use smithay::reexports::wayland_protocols_misc::zwp_virtual_keyboard_v1::server::{
    zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1,
    zwp_virtual_keyboard_v1::ZwpVirtualKeyboardV1,
};
use smithay::reexports::wayland_server::{Dispatch, DisplayHandle, GlobalDispatch};
use smithay::wayland::GlobalData;
use smithay::wayland::virtual_keyboard::{
    VirtualKeyboardManagerGlobalData, VirtualKeyboardManagerState, VirtualKeyboardUserData,
};

/// `zwp_virtual_keyboard_manager_v1`.
#[derive(Debug)]
pub struct VirtualKeyboardPart {
    /// smithay's state.
    pub state: VirtualKeyboardManagerState,
}

impl VirtualKeyboardPart {
    /// Creates the global (version 1), filtered by the I7 table.
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> VirtualKeyboardPart
    where
        D: GlobalDispatch<ZwpVirtualKeyboardManagerV1, VirtualKeyboardManagerGlobalData>
            + Dispatch<ZwpVirtualKeyboardManagerV1, GlobalData>
            + Dispatch<ZwpVirtualKeyboardV1, VirtualKeyboardUserData<D>>
            + SeatHandler
            + 'static,
    {
        VirtualKeyboardPart {
            state: VirtualKeyboardManagerState::new::<D, _>(
                display,
                role_filter.for_global(GlobalKind::VirtualKeyboard),
            ),
        }
    }
}

/// A virtual keyboard sent a key: an `Event::Input` with `Origin::Virtual(Keyboard)`.
pub fn on_vk_key(
    part: &mut VirtualKeyboardPart,
    code: KeyCode,
    edge: Edge,
    at: Mono,
) -> Vec<Event> {
    let _ = (part, code, edge, at);
    todo!("P0-H7: virtual_keyboard::on_vk_key")
}
