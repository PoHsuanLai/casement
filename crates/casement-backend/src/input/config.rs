//! Settings to devices (P1-Z; filled by P1-I1): the stored pointer and keyboard settings as
//! what the libinput seam applies and what the seat compiles.

use super::devices::DeviceSettings;
use casement_core::input::DeviceClass;
use casement_core::settings::{KeyboardSettings, PointerSettings, XkbNames};

/// The settings for a device of `class`: the touchpad profile for a touchpad, the mouse
/// profile for every other pointing device.
///
/// ```
/// use casement_backend::input::device_settings;
/// use casement_core::input::DeviceClass;
/// use casement_core::settings::{Enabled, PointerSettings};
///
/// let touchpad = device_settings(&PointerSettings::defaults(), DeviceClass::Touchpad);
/// assert_eq!(touchpad.natural_scroll, Enabled::On);
/// ```
pub fn device_settings(pointer: &PointerSettings, class: DeviceClass) -> DeviceSettings {
    let profile = match class {
        DeviceClass::Touchpad => &pointer.touchpad,
        _ => &pointer.mouse,
    };
    DeviceSettings::new(
        profile.accel,
        profile.natural_scroll,
        profile.tap,
        profile.scroll_factor,
        profile.disable_while_typing,
    )
}

/// The XKB names a keyboard's keymap is compiled from.
///
/// ```
/// use casement_backend::input::xkb_of;
/// use casement_core::settings::CasementSettings;
///
/// assert_eq!(xkb_of(&CasementSettings::defaults().keyboard).layout, "");
/// ```
pub fn xkb_of(keyboard: &KeyboardSettings) -> XkbNames {
    keyboard.layout.clone()
}
