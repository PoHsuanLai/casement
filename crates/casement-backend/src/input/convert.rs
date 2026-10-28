//! A backend's input event in our own words (P1-T): the one conversion libinput's events get at
//! the edge, written over smithay's input traits so the virtual backend's events run through it
//! in a test (a round trip). The device, instant and origin are stamped by the caller, once, and
//! never changed downstream (I8).

use casement_core::geometry::{LogicalPoint, LogicalRect, SubPoint, SubPx};
use casement_core::input::{
    AxisFrame, AxisSource as CoreSource, ButtonCode, GestureEnd, GestureStep, InputKind, KeyCode,
    PinchDelta, SwitchStep, TouchStep,
};
use casement_protocols::vocab::PressEdge;
use ds_core::vocab::Fraction;
use smithay::backend::input::{
    AbsolutePositionEvent, Axis, AxisSource, ButtonState, GestureBeginEvent, GestureEndEvent,
    GesturePinchUpdateEvent, GestureSwipeUpdateEvent, InputBackend, InputEvent, KeyState,
    KeyboardKeyEvent, PointerAxisEvent, PointerButtonEvent, PointerMotionEvent, Switch,
    SwitchState, SwitchToggleEvent, TouchEvent,
};

/// Sub-pixels per logical pixel.
const SUB_PER_LOGICAL: f64 = 120.0;

/// The XKB keycode offset over an evdev code (what libinput's backend adds).
const XKB_OFFSET: u32 = 8;

/// What `event` is, in our words; none for a device appearing or going, a tablet tool (P2), a
/// hold's update and the backend's own special events. An absolute position (a touchscreen, a
/// tablet in mouse mode) is placed in `extent`, the desktop's union of outputs.
///
/// ```
/// use casement_backend::input::input_kind_of;
/// use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize};
/// use smithay::backend::input::InputEvent;
/// use casement_backend::input::VirtualInput;
///
/// let extent = LogicalRect::new(LogicalPoint { x: 0, y: 0 }, LogicalSize { w: 1920, h: 1080 });
/// let special: InputEvent<VirtualInput> = InputEvent::Special(());
/// assert_eq!(input_kind_of(&special, extent), None);
/// ```
pub fn input_kind_of<B: InputBackend>(
    event: &InputEvent<B>,
    extent: LogicalRect,
) -> Option<InputKind> {
    Some(match event {
        InputEvent::Keyboard { event } => InputKind::Key {
            code: KeyCode(event.key_code().raw().saturating_sub(XKB_OFFSET)),
            edge: match event.state() {
                KeyState::Pressed => PressEdge::Press,
                KeyState::Released => PressEdge::Release,
            },
        },
        InputEvent::PointerMotion { event } => InputKind::Motion {
            delta: sub_point(event.delta_x(), event.delta_y()),
        },
        InputEvent::PointerMotionAbsolute { event } => InputKind::MotionTo {
            at: placed::<B, _>(event, extent),
        },
        InputEvent::PointerButton { event } => InputKind::Button {
            button: ButtonCode(event.button_code()),
            edge: match event.state() {
                ButtonState::Pressed => PressEdge::Press,
                ButtonState::Released => PressEdge::Release,
            },
        },
        InputEvent::PointerAxis { event } => InputKind::Axis(axis_frame::<B, _>(event)),
        InputEvent::GestureSwipeBegin { event } => begin(event.fingers(), InputKind::Swipe),
        InputEvent::GestureSwipeUpdate { event } => InputKind::Swipe(GestureStep::Update(
            sub_point(event.delta_x(), event.delta_y()),
        )),
        InputEvent::GestureSwipeEnd { event } => {
            InputKind::Swipe(GestureStep::End(ended::<B, _>(event)))
        }
        InputEvent::GesturePinchBegin { event } => begin(event.fingers(), InputKind::Pinch),
        InputEvent::GesturePinchUpdate { event } => {
            InputKind::Pinch(GestureStep::Update(PinchDelta {
                scale: Fraction(whole(event.scale() * 1000.0, 0.0, f64::from(u16::MAX)) as u16),
                rotation_millideg: whole(
                    event.rotation() * 1000.0,
                    f64::from(i32::MIN),
                    f64::from(i32::MAX),
                ) as i32,
                delta: sub_point(event.delta_x(), event.delta_y()),
            }))
        }
        InputEvent::GesturePinchEnd { event } => {
            InputKind::Pinch(GestureStep::End(ended::<B, _>(event)))
        }
        InputEvent::GestureHoldBegin { event } => begin(event.fingers(), InputKind::Hold),
        InputEvent::GestureHoldEnd { event } => {
            InputKind::Hold(GestureStep::End(ended::<B, _>(event)))
        }
        InputEvent::TouchDown { event } => InputKind::Touch(TouchStep::Down {
            slot: slot_of::<B, _>(event),
            at: placed::<B, _>(event, extent),
        }),
        InputEvent::TouchMotion { event } => InputKind::Touch(TouchStep::Motion {
            slot: slot_of::<B, _>(event),
            at: placed::<B, _>(event, extent),
        }),
        InputEvent::TouchUp { event } => InputKind::Touch(TouchStep::Up {
            slot: slot_of::<B, _>(event),
        }),
        InputEvent::TouchCancel { .. } => InputKind::Touch(TouchStep::Cancel),
        InputEvent::TouchFrame { .. } => InputKind::Touch(TouchStep::Frame),
        InputEvent::SwitchToggle { event } => InputKind::Switch(switch_step::<B, _>(event)?),
        _ => return None,
    })
}

/// A rounded value clamped to `[low, high]`, as a whole number still held in a float (the
/// caller narrows it: the clamp makes that exact).
fn whole(value: f64, low: f64, high: f64) -> f64 {
    value.round().clamp(low, high)
}

/// A length in logical pixels as sub-pixels.
fn sub_px(value: f64) -> SubPx {
    SubPx(whole(
        value * SUB_PER_LOGICAL,
        f64::from(i32::MIN),
        f64::from(i32::MAX),
    ) as i32)
}

fn sub_point(x: f64, y: f64) -> SubPoint {
    SubPoint {
        x: sub_px(x),
        y: sub_px(y),
    }
}

/// Where an absolute event lands: its position over `extent`.
fn placed<B: InputBackend, E: AbsolutePositionEvent<B>>(
    event: &E,
    extent: LogicalRect,
) -> LogicalPoint {
    let span = |side: u32| i32::try_from(side).unwrap_or(i32::MAX);
    let place = |origin: i32, at: f64| {
        let whole = whole(at, f64::from(i32::MIN), f64::from(i32::MAX)) as i32;
        origin.saturating_add(whole)
    };
    LogicalPoint {
        x: place(extent.origin.x, event.x_transformed(span(extent.size.w))),
        y: place(extent.origin.y, event.y_transformed(span(extent.size.h))),
    }
}

fn slot_of<B: InputBackend, E: TouchEvent<B>>(event: &E) -> u32 {
    u32::try_from(i32::from(event.slot())).unwrap_or(0)
}

fn begin<D>(fingers: u32, kind: fn(GestureStep<D>) -> InputKind) -> InputKind {
    kind(GestureStep::Begin {
        fingers: u8::try_from(fingers).unwrap_or(u8::MAX),
    })
}

fn ended<B: InputBackend, E: GestureEndEvent<B>>(event: &E) -> GestureEnd {
    match event.cancelled() {
        true => GestureEnd::Cancelled,
        false => GestureEnd::Lifted,
    }
}

/// One scroll frame. A continuous source (fingers, a trackpoint) that reports a zero amount has
/// stopped; a wheel's frame always runs.
fn axis_frame<B: InputBackend, E: PointerAxisEvent<B>>(event: &E) -> AxisFrame {
    let (px_x, px_y) = (event.amount(Axis::Horizontal), event.amount(Axis::Vertical));
    let detents = |axis| {
        event.amount_v120(axis).map_or(0, |n| {
            whole(n, f64::from(i32::MIN), f64::from(i32::MAX)) as i32
        })
    };
    let source = match event.source() {
        AxisSource::Wheel => CoreSource::Wheel,
        AxisSource::Finger => CoreSource::Finger,
        AxisSource::Continuous => CoreSource::Continuous,
        AxisSource::WheelTilt => CoreSource::WheelTilt,
    };
    let continuous = matches!(source, CoreSource::Finger | CoreSource::Continuous);
    let reported: Vec<f64> = [px_x, px_y].into_iter().flatten().collect();
    let stopped = continuous && !reported.is_empty() && reported.iter().all(|n| *n == 0.0);
    AxisFrame {
        source,
        v120: (detents(Axis::Horizontal), detents(Axis::Vertical)),
        px: sub_point(px_x.unwrap_or(0.0), px_y.unwrap_or(0.0)),
        stopped,
    }
}

/// A switch toggle; none for a switch we have no word for.
fn switch_step<B: InputBackend, E: SwitchToggleEvent<B>>(event: &E) -> Option<SwitchStep> {
    let on = event.state() == SwitchState::On;
    match event.switch()? {
        Switch::Lid => Some(SwitchStep::Lid { closed: on }),
        Switch::TabletMode => Some(SwitchStep::TabletMode { tablet: on }),
        _ => None,
    }
}
