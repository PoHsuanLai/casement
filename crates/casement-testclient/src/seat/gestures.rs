//! zwp_pointer_gestures_v1: swipe, pinch, hold. Filled by P0-H4.
//!
//! The helper asks for the swipe, pinch and hold objects of a pointer; their events reach the
//! transcript through the connection's tap.

use crate::conn::{ClientError, ClientState, TestClient};
use crate::seat::pointer::Pointer;
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols::wp::pointer_gestures::zv1::client::zwp_pointer_gesture_hold_v1::{
    self, ZwpPointerGestureHoldV1,
};
use wayland_protocols::wp::pointer_gestures::zv1::client::zwp_pointer_gesture_pinch_v1::{
    self, ZwpPointerGesturePinchV1,
};
use wayland_protocols::wp::pointer_gestures::zv1::client::zwp_pointer_gesture_swipe_v1::{
    self, ZwpPointerGestureSwipeV1,
};
use wayland_protocols::wp::pointer_gestures::zv1::client::zwp_pointer_gestures_v1::ZwpPointerGesturesV1;

/// The user data of the gesture objects: nothing, the transcript has the events.
#[derive(Debug, Clone, Copy, Default)]
pub struct GesturePlain;

/// The three gesture objects of one pointer.
#[derive(Debug)]
pub struct Gestures {
    swipe: ZwpPointerGestureSwipeV1,
    pinch: ZwpPointerGesturePinchV1,
    hold: ZwpPointerGestureHoldV1,
}

impl Gestures {
    /// Binds the manager (version 3 when offered) and gets every gesture of `pointer`.
    pub fn bind(client: &TestClient, pointer: &Pointer) -> Result<Gestures, ClientError> {
        let manager: ZwpPointerGesturesV1 = client.bind(1..=3, GesturePlain)?;
        let qh = client.qh();
        let swipe = manager.get_swipe_gesture(pointer.pointer(), &qh, GesturePlain);
        let pinch = manager.get_pinch_gesture(pointer.pointer(), &qh, GesturePlain);
        let hold = manager.get_hold_gesture(pointer.pointer(), &qh, GesturePlain);
        let _ = client.connection().flush();
        Ok(Gestures { swipe, pinch, hold })
    }

    /// The swipe object.
    pub fn swipe(&self) -> &ZwpPointerGestureSwipeV1 {
        &self.swipe
    }

    /// The pinch object.
    pub fn pinch(&self) -> &ZwpPointerGesturePinchV1 {
        &self.pinch
    }

    /// The hold object.
    pub fn hold(&self) -> &ZwpPointerGestureHoldV1 {
        &self.hold
    }
}

macro_rules! plain {
    ($($object:ty => $event:ty),* $(,)?) => {$(
        impl Dispatch<$object, GesturePlain> for ClientState {
            fn event(
                _: &mut Self,
                _: &$object,
                _: $event,
                _: &GesturePlain,
                _: &Connection,
                _: &QueueHandle<Self>,
            ) {
            }
        }
    )*};
}

plain! {
    ZwpPointerGesturesV1 => <ZwpPointerGesturesV1 as Proxy>::Event,
    ZwpPointerGestureSwipeV1 => zwp_pointer_gesture_swipe_v1::Event,
    ZwpPointerGesturePinchV1 => zwp_pointer_gesture_pinch_v1::Event,
    ZwpPointerGestureHoldV1 => zwp_pointer_gesture_hold_v1::Event,
}
