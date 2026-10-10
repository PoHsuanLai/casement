//! Settings to libinput device configuration (P1-I1), asserted at the seam with the recorder.

use casement_backend::input::{AccelProfile, device_settings, xkb_of};
use casement_core::input::DeviceClass;
use casement_core::settings::{CasementSettings, Enabled, PointerSettings};

#[test]
fn a_touchpad_gets_the_touchpad_profile_and_a_mouse_the_mouse_profile() {
    let settings = PointerSettings::defaults();
    let touchpad = device_settings(&settings, DeviceClass::Touchpad);
    let mouse = device_settings(&settings, DeviceClass::Pointer);
    assert_eq!(touchpad.natural_scroll, Enabled::On);
    assert_eq!(mouse.natural_scroll, Enabled::Off);
    assert_eq!(touchpad.accel, AccelProfile::Adaptive);
}

#[test]
fn a_changed_profile_reaches_the_device_as_applied() {
    let mut settings = PointerSettings::defaults();
    settings.touchpad.tap = Enabled::Off;
    settings.mouse.tap = Enabled::On;
    let applied = device_settings(&settings, DeviceClass::Touchpad);
    assert_eq!(applied.tap, Enabled::Off);
    assert_eq!(applied.scroll_factor, settings.touchpad.scroll_factor);
    assert_eq!(
        device_settings(&settings, DeviceClass::Pointer).tap,
        Enabled::On,
        "the mouse profile is its own"
    );
}

#[test]
fn the_xkb_names_are_the_stored_ones() {
    let mut settings = CasementSettings::defaults();
    settings.keyboard.layout.layout = "us,de".to_owned();
    settings.keyboard.layout.options = "grp:alt_shift_toggle".to_owned();
    let names = xkb_of(&settings.keyboard);
    assert_eq!(names.layout, "us,de");
    assert_eq!(names.options, "grp:alt_shift_toggle");
    assert_eq!(names.rules, "");
}
