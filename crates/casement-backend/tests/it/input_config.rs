//! Settings to libinput device configuration (P1-I1), asserted at the seam with the recorder.

use casement_backend::input::{AccelProfile, PresentDevices, device_settings, xkb_of};
use casement_core::DeviceId;
use casement_core::input::DeviceClass;
use casement_core::settings::{CasementSettings, Enabled, PointerSettings};

#[test]
fn a_touchpad_gets_the_touchpad_profile_and_a_mouse_the_mouse_profile() {
    let settings = PointerSettings::defaults();
    let touchpad = device_settings(&settings, DeviceClass::Touchpad);
    let mouse = device_settings(&settings, DeviceClass::Pointer);
    assert_eq!(touchpad.natural_scroll, Enabled::On);
    assert_eq!(mouse.natural_scroll, Enabled::On);
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

#[test]
fn every_pointing_class_gets_natural_scroll_from_the_defaults() {
    let settings = PointerSettings::defaults();
    // A mouse and a trackball are both `Pointer`; libinput calls a pad with finger taps a touchpad.
    let table = [
        (DeviceClass::Pointer, Enabled::On),
        (DeviceClass::Touchpad, Enabled::On),
    ];
    for (class, natural) in table {
        assert_eq!(
            device_settings(&settings, class).natural_scroll,
            natural,
            "{class:?}"
        );
    }
}

#[test]
fn turning_natural_scroll_off_for_mice_leaves_the_touchpad_natural() {
    let mut settings = PointerSettings::defaults();
    settings.mouse.natural_scroll = Enabled::Off;
    assert_eq!(
        device_settings(&settings, DeviceClass::Pointer).natural_scroll,
        Enabled::Off
    );
    assert_eq!(
        device_settings(&settings, DeviceClass::Touchpad).natural_scroll,
        Enabled::On
    );
}

const MOUSE: DeviceId = DeviceId(1);
const PAD: DeviceId = DeviceId(2);
const KEYS: DeviceId = DeviceId(3);

/// A seat with a mouse, a touchpad and a keyboard, the pointing two configured from `applied`.
fn seat_configured_from(applied: &PointerSettings) -> PresentDevices {
    let mut present = PresentDevices::default();
    present.appeared(MOUSE, DeviceClass::Pointer);
    present.appeared(PAD, DeviceClass::Touchpad);
    present.appeared(KEYS, DeviceClass::Keyboard);
    for (device, class) in [(MOUSE, DeviceClass::Pointer), (PAD, DeviceClass::Touchpad)] {
        present.applied(device, device_settings(applied, class));
    }
    present
}

/// Each row: how the stored settings change, and which devices are told (only a device whose
/// own profile changed, never a keyboard).
#[test]
fn a_settings_change_reaches_exactly_the_devices_it_changes() {
    type Change = fn(&mut PointerSettings);
    let table: &[(&str, Change, &[DeviceId])] = &[
        ("nothing changed", |_| {}, &[]),
        (
            "mouse natural scroll",
            |s| s.mouse.natural_scroll = Enabled::Off,
            &[MOUSE],
        ),
        ("touchpad tap", |s| s.touchpad.tap = Enabled::Off, &[PAD]),
        (
            "touchpad speed curve",
            |s| s.touchpad.accel = AccelProfile::Flat,
            &[PAD],
        ),
        (
            "mouse disable while typing",
            |s| s.mouse.disable_while_typing = Enabled::On,
            &[MOUSE],
        ),
        (
            "both profiles",
            |s| {
                s.mouse.tap = Enabled::On;
                s.touchpad.natural_scroll = Enabled::Off;
            },
            &[MOUSE, PAD],
        ),
    ];
    for (what, change, told) in table {
        let present = seat_configured_from(&PointerSettings::defaults());
        let mut now = PointerSettings::defaults();
        change(&mut now);
        let ids: Vec<DeviceId> = present
            .changes(&now)
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        assert_eq!(ids.as_slice(), *told, "{what}");
    }
}

#[test]
fn a_change_carries_the_new_settings_of_the_devices_class() {
    let present = seat_configured_from(&PointerSettings::defaults());
    let mut now = PointerSettings::defaults();
    now.mouse.natural_scroll = Enabled::Off;
    now.touchpad.tap = Enabled::Off;
    let changes = present.changes(&now);
    assert_eq!(
        changes,
        vec![
            (MOUSE, device_settings(&now, DeviceClass::Pointer)),
            (PAD, device_settings(&now, DeviceClass::Touchpad)),
        ]
    );
}

#[test]
fn a_device_that_took_nothing_yet_gets_the_settings_and_one_that_left_does_not() {
    let mut present = PresentDevices::default();
    present.appeared(MOUSE, DeviceClass::Pointer);
    present.appeared(PAD, DeviceClass::Touchpad);
    present.gone(PAD);
    let ids: Vec<DeviceId> = present
        .changes(&PointerSettings::defaults())
        .into_iter()
        .map(|(id, _)| id)
        .collect();
    assert_eq!(ids, vec![MOUSE]);
}

#[test]
fn once_applied_the_same_settings_change_nothing() {
    let mut now = PointerSettings::defaults();
    now.mouse.natural_scroll = Enabled::Off;
    let mut present = seat_configured_from(&PointerSettings::defaults());
    for (id, settings) in present.changes(&now) {
        present.applied(id, settings);
    }
    assert_eq!(present.changes(&now), vec![]);
}
