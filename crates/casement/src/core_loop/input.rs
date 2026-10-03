//! The backend's queued input in our own words: what each virtual event becomes at the edge
//! (the same conversion libinput's events get at P1: device, instant and origin are read
//! here, once, and never changed downstream, I8).

use casement_backend::input::{EventStamp, VirtualInputEvent};
use casement_core::input::{GestureStep, InputEvent, InputKind};
use smithay::backend::input::InputEvent as Backend;

/// `event` as the model's input event; none for a device appearing or going (nothing
/// happened to the desktop) and for a hold's update (a hold has none on the wire).
pub(super) fn event_of(event: &VirtualInputEvent) -> Option<InputEvent> {
    let (stamp, kind) = match event {
        Backend::Keyboard { event } => (
            &event.stamp,
            InputKind::Key {
                code: event.code,
                edge: event.edge,
            },
        ),
        Backend::PointerMotion { event } => {
            (&event.stamp, InputKind::Motion { delta: event.delta })
        }
        Backend::PointerMotionAbsolute { event } => {
            (&event.stamp, InputKind::MotionTo { at: event.at })
        }
        Backend::PointerButton { event } => (
            &event.stamp,
            InputKind::Button {
                button: event.button,
                edge: event.edge,
            },
        ),
        Backend::PointerAxis { event } => (&event.stamp, InputKind::Axis(event.frame)),
        Backend::GestureSwipeBegin { event } => {
            (&event.stamp, begin(event.fingers, InputKind::Swipe))
        }
        Backend::GestureSwipeUpdate { event } => (
            &event.stamp,
            InputKind::Swipe(GestureStep::Update(event.delta)),
        ),
        Backend::GestureSwipeEnd { event } => {
            (&event.stamp, InputKind::Swipe(GestureStep::End(event.end)))
        }
        Backend::GesturePinchBegin { event } => {
            (&event.stamp, begin(event.fingers, InputKind::Pinch))
        }
        Backend::GesturePinchUpdate { event } => (
            &event.stamp,
            InputKind::Pinch(GestureStep::Update(event.delta)),
        ),
        Backend::GesturePinchEnd { event } => {
            (&event.stamp, InputKind::Pinch(GestureStep::End(event.end)))
        }
        Backend::GestureHoldBegin { event } => {
            (&event.stamp, begin(event.fingers, InputKind::Hold))
        }
        Backend::GestureHoldEnd { event } => {
            (&event.stamp, InputKind::Hold(GestureStep::End(event.end)))
        }
        Backend::TouchDown { event }
        | Backend::TouchMotion { event }
        | Backend::TouchUp { event }
        | Backend::TouchCancel { event }
        | Backend::TouchFrame { event } => (&event.stamp, InputKind::Touch(event.step)),
        Backend::SwitchToggle { event } => (&event.stamp, InputKind::Switch(event.step)),
        Backend::DeviceAdded { .. }
        | Backend::DeviceRemoved { .. }
        | Backend::TabletToolAxis { .. }
        | Backend::TabletToolProximity { .. }
        | Backend::TabletToolTip { .. }
        | Backend::TabletToolButton { .. }
        | Backend::Special(()) => return None,
    };
    Some(stamped(stamp, kind))
}

/// A gesture's begin step as `kind`'s variant.
fn begin<D>(fingers: u8, kind: fn(GestureStep<D>) -> InputKind) -> InputKind {
    kind(GestureStep::Begin { fingers })
}

/// `kind` with the device, origin and instant of `stamp`.
fn stamped(stamp: &EventStamp, kind: InputKind) -> InputEvent {
    InputEvent {
        origin: stamp.device.spec.origin,
        device: stamp.device.id,
        at: stamp.at,
        kind,
    }
}
