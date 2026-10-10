//! Settings to devices (P1-Z; filled by P1-I1): the stored pointer and keyboard settings as
//! what the libinput seam applies and what the seat compiles.

use super::devices::DeviceSettings;
use casement_core::input::DeviceClass;
use casement_core::settings::{KeyboardSettings, PointerSettings, XkbNames};

/// The settings for a device of `class`: the touchpad profile for a touchpad, the mouse
/// profile for every other pointing device.
pub fn device_settings(pointer: &PointerSettings, class: DeviceClass) -> DeviceSettings {
    let _ = (pointer, class);
    todo!("P1-I1: device_settings")
}

/// The XKB names a keyboard's keymap is compiled from.
pub fn xkb_of(keyboard: &KeyboardSettings) -> XkbNames {
    let _ = keyboard;
    todo!("P1-I1: xkb_of")
}
