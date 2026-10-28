//! One input event, in our own words: what every backend's events become at the edge.

use super::{ButtonCode, KeyCode, Origin};
use crate::geometry::{LogicalPoint, SubPoint};
use crate::ids::DeviceId;
use crate::time::Mono;
use casement_protocols::vocab::PressEdge;
use ds_core::vocab::Fraction;

/// One input event: where from, which device, when, and what.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputEvent {
    /// Where it came from.
    pub origin: Origin,
    /// The device.
    pub device: DeviceId,
    /// When it happened.
    pub at: Mono,
    /// What happened.
    pub kind: InputKind,
}

/// What an input event is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputKind {
    /// A key went down or up.
    Key {
        /// The key.
        code: KeyCode,
        /// Down or up.
        edge: PressEdge,
    },
    /// The pointer moved by this much.
    Motion {
        /// The movement.
        delta: SubPoint,
    },
    /// The pointer moved to this point (a tablet, a test).
    MotionTo {
        /// The point, in global logical coordinates.
        at: LogicalPoint,
    },
    /// A button went down or up.
    Button {
        /// The button.
        button: ButtonCode,
        /// Down or up.
        edge: PressEdge,
    },
    /// One scroll frame.
    Axis(AxisFrame),
    /// A swipe gesture's step.
    Swipe(GestureStep<SubPoint>),
    /// A pinch gesture's step.
    Pinch(GestureStep<PinchDelta>),
    /// A hold gesture's step.
    Hold(GestureStep<()>),
    /// A touch step.
    Touch(TouchStep),
    /// A switch changed.
    Switch(SwitchStep),
}

/// One scroll frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AxisFrame {
    /// What scrolled.
    pub source: AxisSource,
    /// Wheel detents in 120ths, horizontal then vertical (`axis_value120`).
    pub v120: (i32, i32),
    /// The distance, in sub-pixels.
    pub px: SubPoint,
    /// Whether the scroll stopped (kinetic scrolling ends here).
    pub stopped: bool,
}

/// What produced a scroll.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AxisSource {
    /// A wheel with detents.
    Wheel,
    /// Fingers on a touchpad.
    Finger,
    /// A continuous device (a trackpoint).
    Continuous,
    /// A tilted wheel.
    WheelTilt,
}

/// One step of a touchpad gesture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GestureStep<D> {
    /// It began with this many fingers.
    Begin {
        /// The finger count.
        fingers: u8,
    },
    /// It moved.
    Update(D),
    /// It ended.
    End(GestureEnd),
}

/// How a gesture ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GestureEnd {
    /// The fingers lifted.
    Lifted,
    /// It was cancelled.
    Cancelled,
}

/// One pinch update.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PinchDelta {
    /// The scale since the pinch began (1000 is unchanged; not clamped).
    pub scale: Fraction,
    /// The rotation since the last update, in thousandths of a degree.
    pub rotation_millideg: i32,
    /// The centre's movement.
    pub delta: SubPoint,
}

/// One touchscreen step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TouchStep {
    /// A finger touched down.
    Down {
        /// Its slot.
        slot: u32,
        /// Where.
        at: LogicalPoint,
    },
    /// A finger moved.
    Motion {
        /// Its slot.
        slot: u32,
        /// Where to.
        at: LogicalPoint,
    },
    /// A finger lifted.
    Up {
        /// Its slot.
        slot: u32,
    },
    /// The end of one touch frame.
    Frame,
    /// The touch sequence was cancelled.
    Cancel,
}

/// A switch's change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SwitchStep {
    /// The lid is now closed (`closed`) or open.
    Lid {
        /// Whether it is closed.
        closed: bool,
    },
    /// Tablet mode is now on (`tablet`: a tablet) or off (a laptop).
    TabletMode {
        /// Whether it is a tablet.
        tablet: bool,
    },
}
