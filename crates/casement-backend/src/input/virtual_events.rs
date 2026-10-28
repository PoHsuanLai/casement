//! The virtual backend's event types: each carries its device, its instant on the session
//! clock and its data in our units, and answers smithay's event traits from them.

use super::virtual_input::{VirtualDevice, VirtualInput};
use casement_core::geometry::{LogicalPoint, SubPoint, SubPx};
use casement_core::input::{
    AxisFrame, AxisSource as CoreSource, ButtonCode, GestureEnd, KeyCode, PinchDelta, SwitchStep,
    TouchStep,
};
use casement_core::time::Mono;
use casement_protocols::vocab::PressEdge;
use smithay::backend::input::{
    AbsolutePositionEvent, Axis, AxisRelativeDirection, AxisSource, ButtonState, Event,
    GestureBeginEvent, GestureEndEvent, GestureHoldBeginEvent, GestureHoldEndEvent,
    GesturePinchBeginEvent, GesturePinchEndEvent, GesturePinchUpdateEvent, GestureSwipeBeginEvent,
    GestureSwipeEndEvent, GestureSwipeUpdateEvent, InputTime, KeyState, KeyboardKeyEvent, Keycode,
    PointerAxisEvent, PointerButtonEvent, PointerMotionAbsoluteEvent, PointerMotionEvent, Switch,
    SwitchState, SwitchToggleEvent, TouchCancelEvent, TouchDownEvent, TouchEvent, TouchFrameEvent,
    TouchMotionEvent, TouchSlot, TouchUpEvent,
};

/// The device and instant every virtual event carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventStamp {
    /// The device.
    pub device: VirtualDevice,
    /// When, on the session clock.
    pub at: Mono,
}

impl EventStamp {
    /// The instant as smithay's input time (microseconds on the session clock).
    fn input_time(&self) -> InputTime {
        InputTime::from_micros(u64::try_from(self.at.0.as_micros()).unwrap_or(u64::MAX))
    }
}

/// A length in sub-pixels as logical pixels.
fn px(sub: SubPx) -> f64 {
    f64::from(sub.0) / SUB_PER_LOGICAL
}

/// Sub-pixels per logical pixel (`casement_core::geometry::SubPx`).
const SUB_PER_LOGICAL: f64 = 120.0;

/// The XKB keycode of an evdev code (the offset libinput's backend adds).
const XKB_OFFSET: u32 = 8;

macro_rules! event_impl {
    ($ty:ident) => {
        impl Event<VirtualInput> for $ty {
            fn time(&self) -> InputTime {
                self.stamp.input_time()
            }

            fn device(&self) -> VirtualDevice {
                self.stamp.device.clone()
            }
        }
    };
}

/// A key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtualKey {
    /// Device and instant.
    pub stamp: EventStamp,
    /// The key.
    pub code: KeyCode,
    /// Down or up.
    pub edge: PressEdge,
    /// How many keys are down on every device after this event (smithay's `count`).
    pub count: u32,
}
event_impl!(VirtualKey);

impl KeyboardKeyEvent<VirtualInput> for VirtualKey {
    fn key_code(&self) -> Keycode {
        Keycode::new(self.code.0.saturating_add(XKB_OFFSET))
    }

    fn state(&self) -> KeyState {
        match self.edge {
            PressEdge::Press => KeyState::Pressed,
            PressEdge::Release => KeyState::Released,
        }
    }

    fn count(&self) -> u32 {
        self.count
    }
}

/// A relative pointer motion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtualMotion {
    /// Device and instant.
    pub stamp: EventStamp,
    /// The movement.
    pub delta: SubPoint,
}
event_impl!(VirtualMotion);

impl PointerMotionEvent<VirtualInput> for VirtualMotion {
    fn delta_x(&self) -> f64 {
        px(self.delta.x)
    }

    fn delta_y(&self) -> f64 {
        px(self.delta.y)
    }

    fn delta_x_unaccel(&self) -> f64 {
        px(self.delta.x)
    }

    fn delta_y_unaccel(&self) -> f64 {
        px(self.delta.y)
    }
}

/// An absolute pointer motion (a test placing the pointer, a tablet in mouse mode).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtualMotionTo {
    /// Device and instant.
    pub stamp: EventStamp,
    /// The point, in global logical coordinates.
    pub at: LogicalPoint,
}
event_impl!(VirtualMotionTo);

impl AbsolutePositionEvent<VirtualInput> for VirtualMotionTo {
    fn x(&self) -> f64 {
        f64::from(self.at.x)
    }

    fn y(&self) -> f64 {
        f64::from(self.at.y)
    }

    fn x_transformed(&self, width: i32) -> f64 {
        let _ = width;
        f64::from(self.at.x)
    }

    fn y_transformed(&self, height: i32) -> f64 {
        let _ = height;
        f64::from(self.at.y)
    }
}

impl PointerMotionAbsoluteEvent<VirtualInput> for VirtualMotionTo {}

/// A pointer button.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtualButton {
    /// Device and instant.
    pub stamp: EventStamp,
    /// The button.
    pub button: ButtonCode,
    /// Down or up.
    pub edge: PressEdge,
}
event_impl!(VirtualButton);

impl PointerButtonEvent<VirtualInput> for VirtualButton {
    fn button_code(&self) -> u32 {
        self.button.0
    }

    fn state(&self) -> ButtonState {
        match self.edge {
            PressEdge::Press => ButtonState::Pressed,
            PressEdge::Release => ButtonState::Released,
        }
    }
}

/// One scroll frame (`value120` and the source always carried: SP1 B1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtualAxis {
    /// Device and instant.
    pub stamp: EventStamp,
    /// The frame.
    pub frame: AxisFrame,
}
event_impl!(VirtualAxis);

impl PointerAxisEvent<VirtualInput> for VirtualAxis {
    fn amount(&self, axis: Axis) -> Option<f64> {
        let sub = match axis {
            Axis::Horizontal => self.frame.px.x,
            Axis::Vertical => self.frame.px.y,
        };
        let continuous = matches!(
            self.frame.source,
            CoreSource::Finger | CoreSource::Continuous
        );
        match (continuous, self.frame.stopped) {
            (true, true) => Some(0.0),
            (true, false) => Some(px(sub)),
            (false, _) => (sub.0 != 0).then(|| px(sub)),
        }
    }

    fn amount_v120(&self, axis: Axis) -> Option<f64> {
        let detents = match axis {
            Axis::Horizontal => self.frame.v120.0,
            Axis::Vertical => self.frame.v120.1,
        };
        let discrete = matches!(self.frame.source, CoreSource::Wheel | CoreSource::WheelTilt);
        (discrete || detents != 0).then(|| f64::from(detents))
    }

    fn source(&self) -> AxisSource {
        match self.frame.source {
            CoreSource::Wheel => AxisSource::Wheel,
            CoreSource::Finger => AxisSource::Finger,
            CoreSource::Continuous => AxisSource::Continuous,
            CoreSource::WheelTilt => AxisSource::WheelTilt,
        }
    }

    fn relative_direction(&self, axis: Axis) -> AxisRelativeDirection {
        let _ = axis;
        AxisRelativeDirection::Identical
    }
}

/// A gesture began (swipe, pinch or hold).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtualGestureBegin {
    /// Device and instant.
    pub stamp: EventStamp,
    /// The finger count.
    pub fingers: u8,
}
event_impl!(VirtualGestureBegin);

impl GestureBeginEvent<VirtualInput> for VirtualGestureBegin {
    fn fingers(&self) -> u32 {
        u32::from(self.fingers)
    }
}

impl GestureSwipeBeginEvent<VirtualInput> for VirtualGestureBegin {}
impl GesturePinchBeginEvent<VirtualInput> for VirtualGestureBegin {}
impl GestureHoldBeginEvent<VirtualInput> for VirtualGestureBegin {}

/// A gesture ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtualGestureEnd {
    /// Device and instant.
    pub stamp: EventStamp,
    /// How it ended.
    pub end: GestureEnd,
}
event_impl!(VirtualGestureEnd);

impl GestureEndEvent<VirtualInput> for VirtualGestureEnd {
    fn cancelled(&self) -> bool {
        self.end == GestureEnd::Cancelled
    }
}

impl GestureSwipeEndEvent<VirtualInput> for VirtualGestureEnd {}
impl GesturePinchEndEvent<VirtualInput> for VirtualGestureEnd {}
impl GestureHoldEndEvent<VirtualInput> for VirtualGestureEnd {}

/// A swipe moved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtualSwipeUpdate {
    /// Device and instant.
    pub stamp: EventStamp,
    /// The movement.
    pub delta: SubPoint,
}
event_impl!(VirtualSwipeUpdate);

impl GestureSwipeUpdateEvent<VirtualInput> for VirtualSwipeUpdate {
    fn delta_x(&self) -> f64 {
        px(self.delta.x)
    }

    fn delta_y(&self) -> f64 {
        px(self.delta.y)
    }
}

/// A pinch moved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtualPinchUpdate {
    /// Device and instant.
    pub stamp: EventStamp,
    /// The step.
    pub delta: PinchDelta,
}
event_impl!(VirtualPinchUpdate);

impl GesturePinchUpdateEvent<VirtualInput> for VirtualPinchUpdate {
    fn delta_x(&self) -> f64 {
        px(self.delta.delta.x)
    }

    fn delta_y(&self) -> f64 {
        px(self.delta.delta.y)
    }

    fn scale(&self) -> f64 {
        f64::from(self.delta.scale.0) / 1000.0
    }

    fn rotation(&self) -> f64 {
        f64::from(self.delta.rotation_millideg) / 1000.0
    }
}

/// A touchscreen step (down, motion, up, frame or cancel).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtualTouch {
    /// Device and instant.
    pub stamp: EventStamp,
    /// The step.
    pub step: TouchStep,
}
event_impl!(VirtualTouch);

impl VirtualTouch {
    /// Where the finger is, for the steps that have a place.
    fn point(&self) -> Option<LogicalPoint> {
        match self.step {
            TouchStep::Down { at, .. } | TouchStep::Motion { at, .. } => Some(at),
            TouchStep::Up { .. } | TouchStep::Frame | TouchStep::Cancel => None,
        }
    }
}

impl TouchEvent<VirtualInput> for VirtualTouch {
    fn slot(&self) -> TouchSlot {
        match self.step {
            TouchStep::Down { slot, .. }
            | TouchStep::Motion { slot, .. }
            | TouchStep::Up { slot } => TouchSlot::from(Some(slot)),
            TouchStep::Frame | TouchStep::Cancel => TouchSlot::from(None),
        }
    }
}

impl AbsolutePositionEvent<VirtualInput> for VirtualTouch {
    fn x(&self) -> f64 {
        self.point().map_or(0.0, |at| f64::from(at.x))
    }

    fn y(&self) -> f64 {
        self.point().map_or(0.0, |at| f64::from(at.y))
    }

    fn x_transformed(&self, width: i32) -> f64 {
        let _ = width;
        self.x()
    }

    fn y_transformed(&self, height: i32) -> f64 {
        let _ = height;
        self.y()
    }
}

impl TouchDownEvent<VirtualInput> for VirtualTouch {}
impl TouchMotionEvent<VirtualInput> for VirtualTouch {}
impl TouchUpEvent<VirtualInput> for VirtualTouch {}
impl TouchCancelEvent<VirtualInput> for VirtualTouch {}
impl TouchFrameEvent<VirtualInput> for VirtualTouch {}

/// A switch changed (the lid, tablet mode).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtualSwitch {
    /// Device and instant.
    pub stamp: EventStamp,
    /// The change.
    pub step: SwitchStep,
}
event_impl!(VirtualSwitch);

impl SwitchToggleEvent<VirtualInput> for VirtualSwitch {
    fn switch(&self) -> Option<Switch> {
        Some(match self.step {
            SwitchStep::Lid { .. } => Switch::Lid,
            SwitchStep::TabletMode { .. } => Switch::TabletMode,
        })
    }

    fn state(&self) -> SwitchState {
        let (SwitchStep::Lid { closed: on } | SwitchStep::TabletMode { tablet: on }) = self.step;
        match on {
            true => SwitchState::On,
            false => SwitchState::Off,
        }
    }
}
