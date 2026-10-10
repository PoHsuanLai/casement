//! Settings to devices (P1-Z; filled by P1-I1): the stored pointer and keyboard settings as
//! what the libinput seam applies and what the seat compiles.

use super::devices::DeviceSettings;
use casement_core::geometry::{SubPoint, SubPx};
use casement_core::input::{AxisFrame, DeviceClass, InputKind};
use casement_core::settings::{KeyboardSettings, PointerSettings, XkbNames};
use ds_core::vocab::Fraction;

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

/// `kind` with its scroll scaled by `factor` (thousandths of the device's own step, 1000 being
/// unchanged): libinput scrolls by the device's step and has no setting for it, so the
/// compositor scales the axis event where it takes it in. Anything but a scroll is returned
/// as it was.
///
/// ```
/// use casement_backend::input::scaled_scroll;
/// use casement_core::geometry::{SubPoint, SubPx};
/// use casement_core::input::{AxisFrame, AxisSource, AxisStop, InputKind};
/// use ds_core::vocab::Fraction;
///
/// let frame = AxisFrame {
///     source: AxisSource::Wheel,
///     v120: (0, 120),
///     px: SubPoint { x: SubPx(0), y: SubPx(1800) },
///     stop: AxisStop::Running,
/// };
/// let InputKind::Axis(twice) = scaled_scroll(InputKind::Axis(frame), Fraction(2000)) else {
///     panic!("still a scroll");
/// };
/// assert_eq!((twice.v120, twice.px.y), ((0, 240), SubPx(3600)));
/// ```
pub fn scaled_scroll(kind: InputKind, factor: Fraction) -> InputKind {
    let scale = |n: i32| {
        let scaled = i64::from(n) * i64::from(factor.0) / 1000;
        i32::try_from(scaled).unwrap_or(if scaled < 0 { i32::MIN } else { i32::MAX })
    };
    match kind {
        InputKind::Axis(frame) => InputKind::Axis(AxisFrame {
            v120: (scale(frame.v120.0), scale(frame.v120.1)),
            px: SubPoint {
                x: SubPx(scale(frame.px.x.0)),
                y: SubPx(scale(frame.px.y.0)),
            },
            ..frame
        }),
        other => other,
    }
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
