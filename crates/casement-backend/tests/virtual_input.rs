//! The virtual input backend: each pushed event comes out of the same smithay traits libinput's
//! do, in the order pushed, with the device's origin and our units converted (P0-B-headless).
#![cfg(any(feature = "testing", feature = "debug-inject"))]

use casement_backend::input::{VirtualInputEvent, VirtualKey};
use casement_backend::{DeviceName, DeviceSpec, VirtualInput};
use casement_core::DeviceId;
use casement_core::geometry::{LogicalPoint, SubPoint, SubPx};
use casement_core::input::{
    AxisFrame, AxisSource as Source, AxisStop, ButtonCode, DeviceClass, Edge, GestureEnd,
    GestureStep, InputKind, KeyCode, LidState, Origin, PinchDelta, SwitchStep, TouchStep,
};
use casement_core::settings::XkbNames;
use casement_core::time::Mono;
use ds_core::vocab::Fraction;
use smithay::backend::input::{
    AbsolutePositionEvent, Axis, AxisRelativeDirection, AxisSource, ButtonState, Device,
    DeviceCapability, Event, GestureBeginEvent, GestureEndEvent, GesturePinchUpdateEvent,
    GestureSwipeUpdateEvent, InputEvent, KeyState, KeyboardKeyEvent, PointerAxisEvent,
    PointerButtonEvent, PointerMotionEvent, Switch, SwitchState, SwitchToggleEvent, TouchEvent,
};
use std::time::Duration;

fn xkb() -> XkbNames {
    XkbNames {
        rules: String::new(),
        model: String::new(),
        layout: "us".to_owned(),
        variant: String::new(),
        options: String::new(),
    }
}

fn device(class: DeviceClass, origin: Origin) -> DeviceSpec {
    DeviceSpec {
        class,
        origin,
        layout: xkb(),
        name: DeviceName(format!("test {class:?}")),
    }
}

fn at(ms: u64) -> Mono {
    Mono(Duration::from_millis(ms))
}

fn px(n: i32) -> SubPx {
    SubPx::of_logical(n)
}

/// One device plugged, its `DeviceAdded` drained.
fn plugged(class: DeviceClass) -> (VirtualInput, DeviceId) {
    let mut input = VirtualInput::empty();
    let id = input.plug(device(class, Origin::Test));
    assert!(matches!(
        input.drain().as_slice(),
        [InputEvent::DeviceAdded { .. }]
    ));
    (input, id)
}

fn key(event: &VirtualInputEvent) -> &VirtualKey {
    match event {
        InputEvent::Keyboard { event } => event,
        other => panic!("not a key: {other:?}"),
    }
}

#[test]
fn plugging_and_unplugging_queue_the_device_events_and_ids_are_never_reused() {
    let mut input = VirtualInput::empty();
    let a = input.plug(device(DeviceClass::Keyboard, Origin::Test));
    let b = input.plug(device(DeviceClass::Pointer, Origin::Test));
    assert_ne!(a, b);
    assert_eq!(input.devices().len(), 2);
    input.unplug(a);
    let again = input.plug(device(DeviceClass::Keyboard, Origin::Test));
    assert!(again != a && again != b, "an id is never reused in a run");
    let events = input.drain();
    assert!(matches!(
        events.as_slice(),
        [
            InputEvent::DeviceAdded { .. },
            InputEvent::DeviceAdded { .. },
            InputEvent::DeviceRemoved { .. },
            InputEvent::DeviceAdded { .. }
        ]
    ));
    assert!(input.drain().is_empty(), "drained once");
}

#[test]
fn a_device_says_what_it_is() {
    let (input, id) = plugged(DeviceClass::Touchpad);
    let device = &input.devices()[&id];
    assert_eq!(device.name(), "test Touchpad");
    assert_eq!(device.id(), format!("virtual-{}", id.0));
    assert!(device.has_capability(DeviceCapability::Pointer));
    assert!(device.has_capability(DeviceCapability::Gesture));
    assert!(!device.has_capability(DeviceCapability::Keyboard));
    assert_eq!(device.usb_id(), None);
    assert_eq!(device.syspath(), None);
}

#[test]
fn events_come_out_in_the_order_pushed_with_their_origin_and_instant() {
    let mut input = VirtualInput::empty();
    let id = input.plug(device(
        DeviceClass::Keyboard,
        Origin::Physical(DeviceClass::Keyboard),
    ));
    input.drain();
    for (n, code) in [30, 48, 46].into_iter().enumerate() {
        let edge = if n % 2 == 0 {
            Edge::Press
        } else {
            Edge::Release
        };
        input.push(
            id,
            InputKind::Key {
                code: KeyCode(code),
                edge,
            },
            at(10 * (n as u64 + 1)),
        );
    }
    let events = input.drain();
    let codes: Vec<u32> = events.iter().map(|e| key(e).code.0).collect();
    assert_eq!(codes, [30, 48, 46]);
    let first = key(&events[0]);
    assert_eq!(
        first.stamp.device.spec.origin,
        Origin::Physical(DeviceClass::Keyboard)
    );
    assert_eq!(first.time().micros(), 10_000);
    assert_eq!(first.device().id, id);
}

#[test]
fn a_key_is_an_xkb_keycode_and_the_count_is_the_keys_held_on_every_device() {
    let mut input = VirtualInput::empty();
    let a = input.plug(device(DeviceClass::Keyboard, Origin::Test));
    let b = input.plug(device(DeviceClass::Keyboard, Origin::Test));
    input.drain();
    let press = |code| InputKind::Key {
        code: KeyCode(code),
        edge: Edge::Press,
    };
    let release = |code| InputKind::Key {
        code: KeyCode(code),
        edge: Edge::Release,
    };
    input.push(a, press(30), at(1));
    input.push(b, press(48), at(2));
    input.push(a, release(30), at(3));
    input.push(b, release(48), at(4));
    let events = input.drain();
    let seen: Vec<(u32, KeyState, u32)> = events
        .iter()
        .map(|e| {
            let k = key(e);
            (u32::from(k.key_code()), k.state(), k.count())
        })
        .collect();
    assert_eq!(
        seen,
        [
            (38, KeyState::Pressed, 1),
            (56, KeyState::Pressed, 2),
            (38, KeyState::Released, 1),
            (56, KeyState::Released, 0),
        ]
    );
}

#[test]
fn input_from_an_unplugged_device_is_dropped_and_its_held_keys_let_go() {
    let (mut input, id) = plugged(DeviceClass::Keyboard);
    input.push(
        id,
        InputKind::Key {
            code: KeyCode(30),
            edge: Edge::Press,
        },
        at(1),
    );
    input.unplug(id);
    input.push(
        id,
        InputKind::Key {
            code: KeyCode(30),
            edge: Edge::Release,
        },
        at(2),
    );
    let events = input.drain();
    assert!(matches!(
        events.as_slice(),
        [
            InputEvent::Keyboard { .. },
            InputEvent::DeviceRemoved { .. }
        ]
    ));
    let other = input.plug(device(DeviceClass::Keyboard, Origin::Test));
    input.drain();
    input.push(
        other,
        InputKind::Key {
            code: KeyCode(31),
            edge: Edge::Press,
        },
        at(3),
    );
    assert_eq!(
        key(&input.drain()[0]).count(),
        1,
        "the unplugged device holds nothing"
    );
}

#[test]
fn pointer_motion_and_buttons_are_in_logical_pixels() {
    let (mut input, id) = plugged(DeviceClass::Pointer);
    input.push(
        id,
        InputKind::Motion {
            delta: SubPoint {
                x: SubPx(60),
                y: SubPx(-180),
            },
        },
        at(1),
    );
    input.push(
        id,
        InputKind::MotionTo {
            at: LogicalPoint { x: 640, y: 360 },
        },
        at(2),
    );
    input.push(
        id,
        InputKind::Button {
            button: ButtonCode(0x110),
            edge: Edge::Press,
        },
        at(3),
    );
    let events = input.drain();
    let InputEvent::PointerMotion { event } = &events[0] else {
        panic!("motion")
    };
    assert_eq!((event.delta_x(), event.delta_y()), (0.5, -1.5));
    assert_eq!(
        (event.delta_x_unaccel(), event.delta_y_unaccel()),
        (0.5, -1.5)
    );
    let InputEvent::PointerMotionAbsolute { event } = &events[1] else {
        panic!("absolute")
    };
    assert_eq!((event.x(), event.y()), (640.0, 360.0));
    assert_eq!(
        (event.x_transformed(1920), event.y_transformed(1080)),
        (640.0, 360.0)
    );
    let InputEvent::PointerButton { event } = &events[2] else {
        panic!("button")
    };
    assert_eq!(event.button_code(), 0x110);
    assert_eq!(event.state(), ButtonState::Pressed);
}

#[test]
fn a_wheel_detent_carries_value120_and_the_source() {
    let (mut input, id) = plugged(DeviceClass::Pointer);
    let frame = AxisFrame {
        source: Source::Wheel,
        v120: (0, -120),
        px: SubPoint {
            x: px(0),
            y: px(-15),
        },
        stop: AxisStop::Running,
    };
    input.push(id, InputKind::Axis(frame), at(1));
    let events = input.drain();
    let InputEvent::PointerAxis { event } = &events[0] else {
        panic!("axis")
    };
    assert_eq!(event.source(), AxisSource::Wheel);
    assert_eq!(event.amount_v120(Axis::Vertical), Some(-120.0));
    assert_eq!(event.amount_v120(Axis::Horizontal), Some(0.0));
    assert_eq!(event.amount(Axis::Vertical), Some(-15.0));
    assert_eq!(event.amount(Axis::Horizontal), None);
    assert_eq!(
        event.relative_direction(Axis::Vertical),
        AxisRelativeDirection::Identical
    );
}

#[test]
fn finger_scrolling_ends_with_a_zero_amount_and_has_no_detents() {
    let (mut input, id) = plugged(DeviceClass::Touchpad);
    let moving = AxisFrame {
        source: Source::Finger,
        v120: (0, 0),
        px: SubPoint { x: px(2), y: px(3) },
        stop: AxisStop::Running,
    };
    input.push(id, InputKind::Axis(moving), at(1));
    input.push(
        id,
        InputKind::Axis(AxisFrame {
            stop: AxisStop::Stopped,
            ..moving
        }),
        at(2),
    );
    let events = input.drain();
    let InputEvent::PointerAxis { event: first } = &events[0] else {
        panic!("axis")
    };
    let InputEvent::PointerAxis { event: last } = &events[1] else {
        panic!("axis")
    };
    assert_eq!(first.amount(Axis::Horizontal), Some(2.0));
    assert_eq!(first.amount(Axis::Vertical), Some(3.0));
    assert_eq!(first.amount_v120(Axis::Vertical), None);
    assert_eq!(last.amount(Axis::Vertical), Some(0.0));
}

#[test]
fn gestures_carry_fingers_deltas_scale_and_how_they_ended() {
    let (mut input, id) = plugged(DeviceClass::Touchpad);
    input.push(
        id,
        InputKind::Swipe(GestureStep::Begin { fingers: 3 }),
        at(1),
    );
    input.push(
        id,
        InputKind::Swipe(GestureStep::Update(SubPoint {
            x: px(4),
            y: px(-2),
        })),
        at(2),
    );
    input.push(
        id,
        InputKind::Swipe(GestureStep::End(GestureEnd::Lifted)),
        at(3),
    );
    let pinch = PinchDelta {
        scale: Fraction(1250),
        rotation_millideg: 15_500,
        delta: SubPoint {
            x: SubPx(30),
            y: SubPx(0),
        },
    };
    input.push(id, InputKind::Pinch(GestureStep::Update(pinch)), at(4));
    input.push(
        id,
        InputKind::Pinch(GestureStep::End(GestureEnd::Cancelled)),
        at(5),
    );
    input.push(
        id,
        InputKind::Hold(GestureStep::Begin { fingers: 2 }),
        at(6),
    );
    input.push(id, InputKind::Hold(GestureStep::Update(())), at(7));
    let events = input.drain();
    assert_eq!(events.len(), 7);
    let InputEvent::GestureSwipeBegin { event } = &events[0] else {
        panic!("swipe begin")
    };
    assert_eq!(event.fingers(), 3);
    let InputEvent::GestureSwipeUpdate { event } = &events[1] else {
        panic!("swipe update")
    };
    assert_eq!((event.delta_x(), event.delta_y()), (4.0, -2.0));
    let InputEvent::GestureSwipeEnd { event } = &events[2] else {
        panic!("swipe end")
    };
    assert!(!event.cancelled());
    let InputEvent::GesturePinchUpdate { event } = &events[3] else {
        panic!("pinch update")
    };
    assert_eq!((event.scale(), event.rotation()), (1.25, 15.5));
    assert_eq!((event.delta_x(), event.delta_y()), (0.25, 0.0));
    let InputEvent::GesturePinchEnd { event } = &events[4] else {
        panic!("pinch end")
    };
    assert!(event.cancelled());
    let InputEvent::GestureHoldBegin { event } = &events[5] else {
        panic!("hold begin")
    };
    assert_eq!(event.fingers(), 2);
    assert!(
        matches!(events[6], InputEvent::Special(())),
        "a hold has no update on the wire"
    );
}

#[test]
fn touch_steps_carry_their_slot_and_place() {
    let (mut input, id) = plugged(DeviceClass::Touch);
    let down = TouchStep::Down {
        slot: 2,
        at: LogicalPoint { x: 100, y: 50 },
    };
    input.push(id, InputKind::Touch(down), at(1));
    input.push(id, InputKind::Touch(TouchStep::Up { slot: 2 }), at(2));
    input.push(id, InputKind::Touch(TouchStep::Frame), at(3));
    let events = input.drain();
    let InputEvent::TouchDown { event } = &events[0] else {
        panic!("down")
    };
    assert_eq!(i32::from(event.slot()), 2);
    assert_eq!((event.x(), event.y()), (100.0, 50.0));
    let InputEvent::TouchUp { event } = &events[1] else {
        panic!("up")
    };
    assert_eq!(i32::from(event.slot()), 2);
    let InputEvent::TouchFrame { .. } = &events[2] else {
        panic!("frame")
    };
}

#[test]
fn a_lid_closing_is_a_switch_turning_on() {
    let (mut input, id) = plugged(DeviceClass::Switch);
    input.push(
        id,
        InputKind::Switch(SwitchStep::Lid(LidState::Closed)),
        at(1),
    );
    input.push(
        id,
        InputKind::Switch(SwitchStep::Lid(LidState::Open)),
        at(2),
    );
    let events = input.drain();
    let states: Vec<(Option<Switch>, SwitchState)> = events
        .iter()
        .map(|e| match e {
            InputEvent::SwitchToggle { event } => (event.switch(), event.state()),
            other => panic!("not a switch: {other:?}"),
        })
        .collect();
    assert_eq!(
        states,
        [
            (Some(Switch::Lid), SwitchState::On),
            (Some(Switch::Lid), SwitchState::Off)
        ]
    );
}
