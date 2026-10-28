//! The one conversion libinput's events get (P1-T), run over the virtual backend's events, which
//! answer smithay's input traits the same way: every kind of event comes back as the kind it
//! was made from.

use casement_backend::DeviceName;
use casement_backend::input::{
    EventStamp, VirtualAxis, VirtualButton, VirtualDevice, VirtualGestureBegin, VirtualGestureEnd,
    VirtualInput, VirtualInputEvent, VirtualKey, VirtualMotion, VirtualMotionTo,
    VirtualPinchUpdate, VirtualSwipeUpdate, VirtualSwitch, VirtualTouch, input_kind_of,
};
use casement_core::DeviceId;
use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize, SubPoint, SubPx};
use casement_core::input::{
    AxisFrame, AxisSource, ButtonCode, DeviceClass, GestureEnd, GestureStep, InputKind, KeyCode,
    Origin, PinchDelta, SwitchStep, TouchStep,
};
use casement_core::settings::CasementSettings;
use casement_core::time::Mono;
use casement_protocols::vocab::PressEdge;
use ds_core::vocab::Fraction;
use smithay::backend::input::InputEvent;
use std::time::Duration;

fn stamp() -> EventStamp {
    EventStamp {
        device: VirtualDevice {
            id: DeviceId(1),
            spec: casement_backend::DeviceSpec {
                class: DeviceClass::Pointer,
                origin: Origin::Physical(DeviceClass::Pointer),
                layout: CasementSettings::defaults().keyboard.layout,
                name: DeviceName("test".to_owned()),
            },
        },
        at: Mono(Duration::from_millis(5)),
    }
}

fn extent() -> LogicalRect {
    LogicalRect::new(
        LogicalPoint { x: 0, y: 0 },
        LogicalSize { w: 1920, h: 1080 },
    )
}

fn sub(x: i32, y: i32) -> SubPoint {
    SubPoint {
        x: SubPx(x),
        y: SubPx(y),
    }
}

/// Each row: an event of the virtual backend and the kind it must read as.
fn rows() -> Vec<(VirtualInputEvent, InputKind)> {
    let key = |edge| InputEvent::Keyboard {
        event: VirtualKey {
            stamp: stamp(),
            code: KeyCode(30),
            edge,
            count: 1,
        },
    };
    let wheel = AxisFrame {
        source: AxisSource::Wheel,
        v120: (0, -120),
        px: sub(0, -1800),
        stopped: false,
    };
    let fingers_up = AxisFrame {
        source: AxisSource::Finger,
        v120: (0, 0),
        px: sub(0, 0),
        stopped: true,
    };
    let fingers = AxisFrame {
        source: AxisSource::Finger,
        v120: (0, 0),
        px: sub(240, -600),
        stopped: false,
    };
    let pinch = PinchDelta {
        scale: Fraction(1500),
        rotation_millideg: -5000,
        delta: sub(120, 240),
    };
    let at = LogicalPoint { x: 300, y: 200 };
    vec![
        (
            key(PressEdge::Press),
            InputKind::Key {
                code: KeyCode(30),
                edge: PressEdge::Press,
            },
        ),
        (
            key(PressEdge::Release),
            InputKind::Key {
                code: KeyCode(30),
                edge: PressEdge::Release,
            },
        ),
        (
            InputEvent::PointerMotion {
                event: VirtualMotion {
                    stamp: stamp(),
                    delta: sub(240, -120),
                },
            },
            InputKind::Motion {
                delta: sub(240, -120),
            },
        ),
        (
            InputEvent::PointerMotionAbsolute {
                event: VirtualMotionTo { stamp: stamp(), at },
            },
            InputKind::MotionTo { at },
        ),
        (
            InputEvent::PointerButton {
                event: VirtualButton {
                    stamp: stamp(),
                    button: ButtonCode(0x110),
                    edge: PressEdge::Press,
                },
            },
            InputKind::Button {
                button: ButtonCode(0x110),
                edge: PressEdge::Press,
            },
        ),
        (
            InputEvent::PointerAxis {
                event: VirtualAxis {
                    stamp: stamp(),
                    frame: wheel,
                },
            },
            InputKind::Axis(wheel),
        ),
        (
            InputEvent::PointerAxis {
                event: VirtualAxis {
                    stamp: stamp(),
                    frame: fingers,
                },
            },
            InputKind::Axis(fingers),
        ),
        (
            InputEvent::PointerAxis {
                event: VirtualAxis {
                    stamp: stamp(),
                    frame: fingers_up,
                },
            },
            InputKind::Axis(fingers_up),
        ),
        (
            InputEvent::GestureSwipeBegin {
                event: VirtualGestureBegin {
                    stamp: stamp(),
                    fingers: 3,
                },
            },
            InputKind::Swipe(GestureStep::Begin { fingers: 3 }),
        ),
        (
            InputEvent::GestureSwipeUpdate {
                event: VirtualSwipeUpdate {
                    stamp: stamp(),
                    delta: sub(120, 0),
                },
            },
            InputKind::Swipe(GestureStep::Update(sub(120, 0))),
        ),
        (
            InputEvent::GestureSwipeEnd {
                event: VirtualGestureEnd {
                    stamp: stamp(),
                    end: GestureEnd::Cancelled,
                },
            },
            InputKind::Swipe(GestureStep::End(GestureEnd::Cancelled)),
        ),
        (
            InputEvent::GesturePinchUpdate {
                event: VirtualPinchUpdate {
                    stamp: stamp(),
                    delta: pinch,
                },
            },
            InputKind::Pinch(GestureStep::Update(pinch)),
        ),
        (
            InputEvent::GestureHoldBegin {
                event: VirtualGestureBegin {
                    stamp: stamp(),
                    fingers: 2,
                },
            },
            InputKind::Hold(GestureStep::Begin { fingers: 2 }),
        ),
        (
            InputEvent::GestureHoldEnd {
                event: VirtualGestureEnd {
                    stamp: stamp(),
                    end: GestureEnd::Lifted,
                },
            },
            InputKind::Hold(GestureStep::End(GestureEnd::Lifted)),
        ),
        (
            InputEvent::TouchDown {
                event: VirtualTouch {
                    stamp: stamp(),
                    step: TouchStep::Down { slot: 2, at },
                },
            },
            InputKind::Touch(TouchStep::Down { slot: 2, at }),
        ),
        (
            InputEvent::TouchMotion {
                event: VirtualTouch {
                    stamp: stamp(),
                    step: TouchStep::Motion { slot: 2, at },
                },
            },
            InputKind::Touch(TouchStep::Motion { slot: 2, at }),
        ),
        (
            InputEvent::TouchUp {
                event: VirtualTouch {
                    stamp: stamp(),
                    step: TouchStep::Up { slot: 2 },
                },
            },
            InputKind::Touch(TouchStep::Up { slot: 2 }),
        ),
        (
            InputEvent::TouchCancel {
                event: VirtualTouch {
                    stamp: stamp(),
                    step: TouchStep::Cancel,
                },
            },
            InputKind::Touch(TouchStep::Cancel),
        ),
        (
            InputEvent::TouchFrame {
                event: VirtualTouch {
                    stamp: stamp(),
                    step: TouchStep::Frame,
                },
            },
            InputKind::Touch(TouchStep::Frame),
        ),
        (
            InputEvent::SwitchToggle {
                event: VirtualSwitch {
                    stamp: stamp(),
                    step: SwitchStep::Lid { closed: true },
                },
            },
            InputKind::Switch(SwitchStep::Lid { closed: true }),
        ),
    ]
}

#[test]
fn every_kind_of_event_reads_back_as_itself() {
    for (event, kind) in rows() {
        assert_eq!(
            input_kind_of::<VirtualInput>(&event, extent()),
            Some(kind.clone()),
            "{kind:?}"
        );
    }
}

#[test]
fn a_device_appearing_or_going_is_no_input() {
    let device = stamp().device;
    let added: VirtualInputEvent = InputEvent::DeviceAdded {
        device: device.clone(),
    };
    let removed: VirtualInputEvent = InputEvent::DeviceRemoved { device };
    assert_eq!(input_kind_of::<VirtualInput>(&added, extent()), None);
    assert_eq!(input_kind_of::<VirtualInput>(&removed, extent()), None);
}
