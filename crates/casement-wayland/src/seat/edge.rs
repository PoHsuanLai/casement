//! The seat's edge: our input units become smithay's. Floats appear only here, because
//! smithay's input handles take `f64` points and the wire carries `wl_fixed` (CONVENTIONS
//! addendum 8: stored values stay integers).

use casement_core::geometry::{LogicalPoint, SubPoint, SubPx};
use casement_core::input::{AxisFrame, AxisSource, ButtonCode, KeyCode};
use casement_core::time::Mono;
use casement_protocols::vocab::PressEdge;
use smithay::backend::input::{Axis, AxisSource as WireSource, ButtonState, InputTime, KeyState};
use smithay::input::keyboard::Keycode;
use smithay::input::pointer::AxisFrame as WireAxisFrame;
use smithay::utils::{Logical, Point};

/// Sub-pixels in one logical pixel (`SubPx`'s unit).
const SUB_PER_LOGICAL: f64 = 120.0;

/// The session instant as smithay's input time, in microseconds.
pub fn time_of(at: Mono) -> InputTime {
    InputTime::from_micros(u64::try_from(at.0.as_micros()).unwrap_or(u64::MAX))
}

/// A sub-pixel length as logical pixels.
pub fn px_of(length: SubPx) -> f64 {
    f64::from(length.0) / SUB_PER_LOGICAL
}

/// A sub-pixel point as smithay's logical point.
pub fn logical_of(point: SubPoint) -> Point<f64, Logical> {
    Point::from((px_of(point.x), px_of(point.y)))
}

/// A whole-pixel point as smithay's logical point.
pub fn whole_of(point: LogicalPoint) -> Point<f64, Logical> {
    Point::from((f64::from(point.x), f64::from(point.y)))
}

/// An evdev key code as the XKB keycode smithay's keyboard takes (evdev plus 8).
pub fn keycode_of(code: KeyCode) -> Keycode {
    Keycode::new(code.0.saturating_add(8))
}

/// Down or up as smithay's key state.
pub fn key_state_of(edge: PressEdge) -> KeyState {
    match edge {
        PressEdge::Press => KeyState::Pressed,
        PressEdge::Release => KeyState::Released,
    }
}

/// Down or up as smithay's button state.
pub fn button_state_of(edge: PressEdge) -> ButtonState {
    match edge {
        PressEdge::Press => ButtonState::Pressed,
        PressEdge::Release => ButtonState::Released,
    }
}

/// An evdev button code as the number on the wire.
pub fn button_of(button: ButtonCode) -> u32 {
    button.0
}

/// What produced a scroll, as smithay's source.
pub fn source_of(source: AxisSource) -> WireSource {
    match source {
        AxisSource::Wheel => WireSource::Wheel,
        AxisSource::Finger => WireSource::Finger,
        AxisSource::Continuous => WireSource::Continuous,
        AxisSource::WheelTilt => WireSource::WheelTilt,
    }
}

/// One scroll frame as smithay's: the source always, `value120` whenever the device has
/// detents (SP1 B1: the engine reads a wheel detent from `axis_value120`, not from pixels),
/// the distance, and the stop on both axes when the scroll ended.
pub fn axis_frame_of(frame: AxisFrame, time: Mono) -> WireAxisFrame {
    let (horizontal, vertical) = frame.v120;
    let with_detents = match (horizontal, vertical) {
        (0, 0) => WireAxisFrame::new(time_of(time)),
        (h, v) => WireAxisFrame::new(time_of(time))
            .v120(Axis::Horizontal, h)
            .v120(Axis::Vertical, v),
    };
    let moved = with_detents
        .source(source_of(frame.source))
        .value(Axis::Horizontal, px_of(frame.px.x))
        .value(Axis::Vertical, px_of(frame.px.y));
    match frame.stopped {
        false => moved,
        true => moved.stop(Axis::Horizontal).stop(Axis::Vertical),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn frame(source: AxisSource, v120: (i32, i32), px: (i32, i32), stopped: bool) -> AxisFrame {
        AxisFrame {
            source,
            v120,
            px: SubPoint {
                x: SubPx(px.0),
                y: SubPx(px.1),
            },
            stopped,
        }
    }

    #[test]
    fn a_wheel_detent_carries_its_source_value120_and_distance() {
        let at = Mono(Duration::from_millis(40));
        let wire = axis_frame_of(frame(AxisSource::Wheel, (0, 120), (0, 1800), false), at);
        assert!(matches!(wire.source, Some(WireSource::Wheel)));
        assert_eq!(wire.v120, Some((0, 120)));
        assert_eq!(wire.axis, (0.0, 15.0));
        assert_eq!(wire.stop, (false, false));
        assert_eq!(wire.time.millis(), 40);
    }

    #[test]
    fn a_finger_scroll_has_no_detents_and_ends_with_a_stop() {
        let at = Mono(Duration::from_millis(7));
        let moving = axis_frame_of(frame(AxisSource::Finger, (0, 0), (60, -120), false), at);
        assert!(matches!(moving.source, Some(WireSource::Finger)));
        assert_eq!(moving.v120, None);
        assert_eq!(moving.axis, (0.5, -1.0));
        let ended = axis_frame_of(frame(AxisSource::Finger, (0, 0), (0, 0), true), at);
        assert_eq!(ended.stop, (true, true));
    }

    #[test]
    fn units_convert_exactly() {
        assert_eq!(px_of(SubPx(240)), 2.0);
        assert_eq!(px_of(SubPx(-60)), -0.5);
        assert_eq!(time_of(Mono(Duration::from_micros(1500))).micros(), 1500);
        assert_eq!(keycode_of(KeyCode(30)).raw(), 38);
        assert_eq!(button_of(ButtonCode(0x110)), 0x110);
        assert_eq!(key_state_of(PressEdge::Press), KeyState::Pressed);
        assert_eq!(button_state_of(PressEdge::Release), ButtonState::Released);
    }
}
