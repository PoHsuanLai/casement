//! `zwp_pointer_gestures_v1`: swipe, pinch and hold, sent by `seat::apply_gesture` once the
//! model leaves a gesture to the client.

use crate::states::GlobalFilter;
use smithay::input::SeatHandler;
use smithay::reexports::wayland_protocols::wp::pointer_gestures::zv1::server::{
    zwp_pointer_gesture_hold_v1::ZwpPointerGestureHoldV1,
    zwp_pointer_gesture_pinch_v1::ZwpPointerGesturePinchV1,
    zwp_pointer_gesture_swipe_v1::ZwpPointerGestureSwipeV1,
    zwp_pointer_gestures_v1::ZwpPointerGesturesV1,
};
use smithay::reexports::wayland_server::{Dispatch, DisplayHandle, GlobalDispatch};
use smithay::wayland::GlobalData;
use smithay::wayland::pointer_gestures::{PointerGestureUserData, PointerGesturesState};

/// `zwp_pointer_gestures_v1`.
#[derive(Debug)]
pub struct PointerGesturesPart {
    /// smithay's state.
    pub state: PointerGesturesState,
}

impl PointerGesturesPart {
    /// Creates the global (version 3). Every connection sees it.
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> PointerGesturesPart
    where
        D: GlobalDispatch<ZwpPointerGesturesV1, GlobalData>
            + Dispatch<ZwpPointerGesturesV1, GlobalData>
            + Dispatch<ZwpPointerGestureSwipeV1, PointerGestureUserData<D>>
            + Dispatch<ZwpPointerGesturePinchV1, PointerGestureUserData<D>>
            + Dispatch<ZwpPointerGestureHoldV1, PointerGestureUserData<D>>
            + SeatHandler
            + 'static,
    {
        let _ = role_filter;
        PointerGesturesPart {
            state: PointerGesturesState::new::<D>(display),
        }
    }
}
