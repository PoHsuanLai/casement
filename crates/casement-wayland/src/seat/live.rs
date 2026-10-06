//! The surface the pointer or a touch point is on, as smithay's input handles hold it. It
//! forwards everything to the `wl_surface` except two things a bare `WlSurface` gets wrong:
//! it never sends `leave` to a surface that is gone or whose client is being torn down (the
//! wire refuses an event naming such an object and wayland-backend panics), and a drag that was dropped does not end with
//! `wl_data_device.leave` (the offer is still the client's to read until it finishes it).

use smithay::backend::input::InputTime;
use smithay::input::dnd::{DndFocus, OfferData, Source};
use smithay::input::pointer::{
    AxisFrame, ButtonEvent, GestureHoldBeginEvent, GestureHoldEndEvent, GesturePinchBeginEvent,
    GesturePinchEndEvent, GesturePinchUpdateEvent, GestureSwipeBeginEvent, GestureSwipeEndEvent,
    GestureSwipeUpdateEvent, MotionEvent, PointerTarget, RelativeMotionEvent,
};
use smithay::input::touch::{DownEvent, OrientationEvent, ShapeEvent, TouchTarget, UpEvent};
use smithay::input::{Seat, SeatHandler, touch};
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::{DisplayHandle, Resource};
use smithay::utils::{IsAlive, Logical, Point, Serial};
use smithay::wayland::seat::WaylandFocus;
use std::borrow::Cow;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// A surface in pointer or touch focus.
#[derive(Debug, Clone)]
pub struct Live {
    surface: WlSurface,
    /// Set when a drop was delivered to it: the drag's `leave` is then not sent.
    dropped: Arc<AtomicBool>,
}

impl From<WlSurface> for Live {
    fn from(surface: WlSurface) -> Live {
        Live {
            surface,
            dropped: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl From<Live> for Option<WlSurface> {
    fn from(live: Live) -> Option<WlSurface> {
        Some(live.surface)
    }
}

impl PartialEq for Live {
    fn eq(&self, other: &Live) -> bool {
        self.surface == other.surface
    }
}

impl Live {
    /// Whether an event naming the surface can be sent: the surface is alive and its client is
    /// still known to the backend (while a client is being torn down the backend has forgotten
    /// it before it destroys the client's objects, and sending an event that names one of them
    /// panics).
    fn reachable(&self) -> bool {
        self.surface.alive() && self.surface.client().is_some()
    }
}

impl IsAlive for Live {
    fn alive(&self) -> bool {
        self.surface.alive()
    }
}

impl WaylandFocus for Live {
    fn wl_surface(&self) -> Option<Cow<'_, WlSurface>> {
        Some(Cow::Borrowed(&self.surface))
    }
}

/// Forwards each listed method to the surface.
macro_rules! forward {
    ($($name:ident($event:ty);)*) => {$(
        fn $name(&self, seat: &Seat<D>, data: &mut D, event: &$event) {
            PointerTarget::$name(&self.surface, seat, data, event)
        }
    )*};
}

impl<D> PointerTarget<D> for Live
where
    D: SeatHandler + 'static,
    WlSurface: PointerTarget<D>,
{
    forward! {
        motion(MotionEvent);
        relative_motion(RelativeMotionEvent);
        button(ButtonEvent);
        gesture_swipe_begin(GestureSwipeBeginEvent);
        gesture_swipe_update(GestureSwipeUpdateEvent);
        gesture_swipe_end(GestureSwipeEndEvent);
        gesture_pinch_begin(GesturePinchBeginEvent);
        gesture_pinch_update(GesturePinchUpdateEvent);
        gesture_pinch_end(GesturePinchEndEvent);
        gesture_hold_begin(GestureHoldBeginEvent);
        gesture_hold_end(GestureHoldEndEvent);
    }

    fn enter(&self, seat: &Seat<D>, data: &mut D, event: &MotionEvent) {
        if self.reachable() {
            PointerTarget::enter(&self.surface, seat, data, event)
        }
    }

    fn axis(&self, seat: &Seat<D>, data: &mut D, frame: AxisFrame) {
        PointerTarget::axis(&self.surface, seat, data, frame)
    }

    fn frame(&self, seat: &Seat<D>, data: &mut D) {
        PointerTarget::frame(&self.surface, seat, data)
    }

    fn leave(&self, seat: &Seat<D>, data: &mut D, serial: Serial, time: InputTime) {
        if self.reachable() {
            PointerTarget::leave(&self.surface, seat, data, serial, time)
        }
    }
}

impl<D> TouchTarget<D> for Live
where
    D: SeatHandler + 'static,
    WlSurface: TouchTarget<D>,
{
    fn down(&self, seat: &Seat<D>, data: &mut D, event: &DownEvent) {
        TouchTarget::down(&self.surface, seat, data, event)
    }

    fn up(&self, seat: &Seat<D>, data: &mut D, event: &UpEvent) {
        TouchTarget::up(&self.surface, seat, data, event)
    }

    fn motion(&self, seat: &Seat<D>, data: &mut D, event: &touch::MotionEvent) {
        TouchTarget::motion(&self.surface, seat, data, event)
    }

    fn frame(&self, seat: &Seat<D>, data: &mut D, marker: touch::FrameMarker) {
        TouchTarget::frame(&self.surface, seat, data, marker)
    }

    fn cancel(&self, seat: &Seat<D>, data: &mut D, marker: touch::FrameMarker) {
        TouchTarget::cancel(&self.surface, seat, data, marker)
    }

    fn shape(&self, seat: &Seat<D>, data: &mut D, event: &ShapeEvent) {
        TouchTarget::shape(&self.surface, seat, data, event)
    }

    fn orientation(&self, seat: &Seat<D>, data: &mut D, event: &OrientationEvent) {
        TouchTarget::orientation(&self.surface, seat, data, event)
    }

    fn last_frame(&self, seat: &Seat<D>, data: &mut D) -> Option<touch::FrameMarker> {
        TouchTarget::last_frame(&self.surface, seat, data)
    }
}

impl<D> DndFocus<D> for Live
where
    D: SeatHandler + 'static,
    WlSurface: DndFocus<D>,
{
    type OfferData<S>
        = <WlSurface as DndFocus<D>>::OfferData<S>
    where
        S: Source;

    fn enter<S: Source>(
        &self,
        data: &mut D,
        dh: &DisplayHandle,
        source: Arc<S>,
        seat: &Seat<D>,
        location: Point<f64, Logical>,
        serial: &Serial,
    ) -> Option<Self::OfferData<S>> {
        self.dropped.store(false, Ordering::Release);
        DndFocus::enter(&self.surface, data, dh, source, seat, location, serial)
    }

    fn motion<S: Source>(
        &self,
        data: &mut D,
        offer: Option<&mut Self::OfferData<S>>,
        seat: &Seat<D>,
        location: Point<f64, Logical>,
        time: InputTime,
    ) {
        DndFocus::motion(&self.surface, data, offer, seat, location, time)
    }

    fn leave<S: Source>(
        &self,
        data: &mut D,
        offer: Option<&mut Self::OfferData<S>>,
        seat: &Seat<D>,
    ) {
        let ended = self.dropped.load(Ordering::Acquire);
        if self.reachable() && !ended {
            DndFocus::leave(&self.surface, data, offer, seat)
        }
    }

    fn drop<S: Source>(
        &self,
        data: &mut D,
        offer: Option<&mut Self::OfferData<S>>,
        seat: &Seat<D>,
    ) {
        let taken = offer.as_deref().is_some_and(OfferData::validated);
        self.dropped.store(taken, Ordering::Release);
        DndFocus::drop(&self.surface, data, offer, seat)
    }
}
