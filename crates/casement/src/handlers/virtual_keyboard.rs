//! `zwp_virtual_keyboard_manager_v1` (P0-H7): smithay forwards virtual keys to the focused
//! client by itself, so casement serves the protocol with its own dispatch and every key
//! enters the model (`casement_wayland::virtual_keyboard`, A36).

use crate::state::State;
use casement_core::time::Mono;
use casement_wayland::virtual_keyboard::{VirtualKeyboardHandler, VirtualKeyboardPart};

impl VirtualKeyboardHandler for State {
    fn virtual_keyboard_part(&mut self) -> &mut VirtualKeyboardPart {
        &mut self.wl.virtual_keyboard
    }

    fn virtual_keyboard_now(&self) -> Mono {
        self.clock.now()
    }
}
