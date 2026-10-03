//! The virtual backend's event types: each carries its device, its instant on the session
//! clock and its data in our units, and answers smithay's event traits from them.

use super::virtual_input::{VirtualDevice, VirtualInput};
use casement_core::geometry::{LogicalPoint, SubPoint};
use casement_core::input::{
    AxisFrame, ButtonCode, Edge, GestureEnd, KeyCode, PinchDelta, SwitchStep, TouchStep,
};
use casement_core::time::Mono;
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

macro_rules! event_impl {
    ($ty:ident) => {
        impl Event<VirtualInput> for $ty {
            fn time(&self) -> InputTime {
                todo!(concat!("P0-B-headless: ", stringify!($ty), "::time"))
            }

            fn device(&self) -> VirtualDevice {
                todo!(concat!("P0-B-headless: ", stringify!($ty), "::device"))
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
    pub edge: Edge,
}
event_impl!(VirtualKey);

impl KeyboardKeyEvent<VirtualInput> for VirtualKey {
    fn key_code(&self) -> Keycode {
        todo!("P0-B-headless: VirtualKey::key_code")
    }

    fn state(&self) -> KeyState {
        todo!("P0-B-headless: VirtualKey::state")
    }

    fn count(&self) -> u32 {
        todo!("P0-B-headless: VirtualKey::count")
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
        todo!("P0-B-headless: VirtualMotion::delta_x")
    }

    fn delta_y(&self) -> f64 {
        todo!("P0-B-headless: VirtualMotion::delta_y")
    }

    fn delta_x_unaccel(&self) -> f64 {
        todo!("P0-B-headless: VirtualMotion::delta_x_unaccel")
    }

    fn delta_y_unaccel(&self) -> f64 {
        todo!("P0-B-headless: VirtualMotion::delta_y_unaccel")
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
        todo!("P0-B-headless: VirtualMotionTo::x")
    }

    fn y(&self) -> f64 {
        todo!("P0-B-headless: VirtualMotionTo::y")
    }

    fn x_transformed(&self, width: i32) -> f64 {
        let _ = width;
        todo!("P0-B-headless: VirtualMotionTo::x_transformed")
    }

    fn y_transformed(&self, height: i32) -> f64 {
        let _ = height;
        todo!("P0-B-headless: VirtualMotionTo::y_transformed")
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
    pub edge: Edge,
}
event_impl!(VirtualButton);

impl PointerButtonEvent<VirtualInput> for VirtualButton {
    fn button_code(&self) -> u32 {
        todo!("P0-B-headless: VirtualButton::button_code")
    }

    fn state(&self) -> ButtonState {
        todo!("P0-B-headless: VirtualButton::state")
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
        let _ = axis;
        todo!("P0-B-headless: VirtualAxis::amount")
    }

    fn amount_v120(&self, axis: Axis) -> Option<f64> {
        let _ = axis;
        todo!("P0-B-headless: VirtualAxis::amount_v120")
    }

    fn source(&self) -> AxisSource {
        todo!("P0-B-headless: VirtualAxis::source")
    }

    fn relative_direction(&self, axis: Axis) -> AxisRelativeDirection {
        let _ = axis;
        todo!("P0-B-headless: VirtualAxis::relative_direction")
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
        todo!("P0-B-headless: VirtualGestureBegin::fingers")
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
        todo!("P0-B-headless: VirtualGestureEnd::cancelled")
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
        todo!("P0-B-headless: VirtualSwipeUpdate::delta_x")
    }

    fn delta_y(&self) -> f64 {
        todo!("P0-B-headless: VirtualSwipeUpdate::delta_y")
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
        todo!("P0-B-headless: VirtualPinchUpdate::delta_x")
    }

    fn delta_y(&self) -> f64 {
        todo!("P0-B-headless: VirtualPinchUpdate::delta_y")
    }

    fn scale(&self) -> f64 {
        todo!("P0-B-headless: VirtualPinchUpdate::scale")
    }

    fn rotation(&self) -> f64 {
        todo!("P0-B-headless: VirtualPinchUpdate::rotation")
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

impl TouchEvent<VirtualInput> for VirtualTouch {
    fn slot(&self) -> TouchSlot {
        todo!("P0-B-headless: VirtualTouch::slot")
    }
}

impl AbsolutePositionEvent<VirtualInput> for VirtualTouch {
    fn x(&self) -> f64 {
        todo!("P0-B-headless: VirtualTouch::x")
    }

    fn y(&self) -> f64 {
        todo!("P0-B-headless: VirtualTouch::y")
    }

    fn x_transformed(&self, width: i32) -> f64 {
        let _ = width;
        todo!("P0-B-headless: VirtualTouch::x_transformed")
    }

    fn y_transformed(&self, height: i32) -> f64 {
        let _ = height;
        todo!("P0-B-headless: VirtualTouch::y_transformed")
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
        todo!("P0-B-headless: VirtualSwitch::switch")
    }

    fn state(&self) -> SwitchState {
        todo!("P0-B-headless: VirtualSwitch::state")
    }
}
