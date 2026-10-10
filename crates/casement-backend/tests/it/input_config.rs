//! Settings to libinput device configuration (P1-I1). Ignored until the lane fills it.

use casement_backend::input::{AccelProfile, device_settings};
use casement_core::input::DeviceClass;
use casement_core::settings::{Enabled, PointerSettings};

#[test]
#[ignore = "P1-I1"]
fn a_touchpad_gets_the_touchpad_profile_and_a_mouse_the_mouse_profile() {
    let settings = PointerSettings::defaults();
    let touchpad = device_settings(&settings, DeviceClass::Touchpad);
    let mouse = device_settings(&settings, DeviceClass::Pointer);
    assert_eq!(touchpad.natural_scroll, Enabled::On);
    assert_eq!(mouse.natural_scroll, Enabled::Off);
    assert_eq!(touchpad.accel, AccelProfile::Adaptive);
}
