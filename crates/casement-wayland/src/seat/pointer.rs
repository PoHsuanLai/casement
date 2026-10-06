//! Pointer motion, buttons, scroll and gestures onto the wire, with pointer constraints applied
//! on the way (a locked pointer stays put and only relative motion arrives; a confined one does
//! not leave its region).

use super::edge::{logical_of, time_of, whole_of};
use super::focus_origin;
use super::hold::{Hold, confined_focus, hold_of, settled};
use super::{GestureDelivery, PointerDelivery, SurfaceAt};
use crate::convert::serial_to;
use crate::states::WaylandFault;
use casement_core::InputSerial;
use casement_core::input::{AxisFrame, GestureEnd, GestureStep, PinchDelta};
use casement_core::time::Mono;
use smithay::backend::input::InputTime;
use smithay::input::pointer::{
    ButtonEvent, GestureHoldBeginEvent, GestureHoldEndEvent, GesturePinchBeginEvent,
    GesturePinchEndEvent, GesturePinchUpdateEvent, GestureSwipeBeginEvent, GestureSwipeEndEvent,
    GestureSwipeUpdateEvent, MotionEvent, PointerHandle, RelativeMotionEvent,
};
use smithay::input::{Seat, SeatHandler};
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::utils::{Logical, Point, SERIAL_COUNTER};
use smithay::wayland::pointer_constraints::{PointerConstraintsHandler, with_pointer_constraint};

fn pair<D: SeatHandler>(focus: Option<SurfaceAt>) -> Option<(D::PointerFocus, Point<f64, Logical>)>
where
    D::PointerFocus: From<WlSurface>,
{
    focus.map(|at| (D::PointerFocus::from(at.surface), whole_of(at.origin)))
}

/// Sends the pointer to `location` over `focus`; a new focus gets `enter` (and the old one
/// `leave`) from smithay, and a constraint waiting on the surface it entered is activated.
fn send_motion<D>(
    pointer: &PointerHandle<D>,
    state: &mut D,
    focus: Option<SurfaceAt>,
    location: Point<f64, Logical>,
    time: InputTime,
) where
    D: SeatHandler + PointerConstraintsHandler + 'static,
    D::PointerFocus: From<WlSurface> + Into<Option<WlSurface>> + Clone + PartialEq,
{
    let before = pointer.current_focus();
    if let Some(at) = &focus {
        focus_origin::told(at);
    }
    pointer.motion(
        state,
        pair::<D>(focus),
        &MotionEvent {
            location,
            serial: SERIAL_COUNTER.next_serial(),
            time,
        },
    );
    let after = pointer.current_focus();
    if before != after
        && let Some(entered) = after.and_then(Into::into)
    {
        with_pointer_constraint::<D, _, _>(&entered, pointer, |constraint| {
            if let Some(constraint) = constraint {
                constraint.activate();
            }
        });
    }
}

/// A surface died: when it holds the pointer the pointer is left over nothing (smithay sends
/// no `leave` to a surface that is gone), until the caller's next refocus finds what is there.
pub fn forget_dead<D>(seat: &Seat<D>, state: &mut D, dead: &WlSurface, time: Mono)
where
    D: SeatHandler + PointerConstraintsHandler + 'static,
    D::PointerFocus: From<WlSurface> + Into<Option<WlSurface>> + Clone + PartialEq,
{
    let Some(pointer) = seat.get_pointer() else {
        return;
    };
    let held: Option<WlSurface> = pointer.current_focus().and_then(Into::into);
    if held.as_ref() == Some(dead) {
        let location = pointer.current_location();
        send_motion(&pointer, state, None, location, time_of(time));
    }
}

/// Delivers a pointer motion or button.
pub fn apply_pointer<D>(
    seat: &Seat<D>,
    state: &mut D,
    delivery: PointerDelivery,
) -> Result<(), WaylandFault>
where
    D: SeatHandler + PointerConstraintsHandler + 'static,
    D::PointerFocus: From<WlSurface> + Into<Option<WlSurface>> + Clone + PartialEq,
{
    let pointer = seat.get_pointer().ok_or(WaylandFault::SurfaceGone)?;
    match delivery {
        PointerDelivery::Motion {
            focus,
            at,
            delta,
            time,
        } => {
            let time = time_of(time);
            let hold = hold_of(&pointer);
            let over = focus.as_ref().map(|at| (&at.surface, whole_of(at.origin)));
            let moved = settled(&hold, over, logical_of(at));
            let relative = relative_of(&focus, delta, time);
            if let Some(location) = moved {
                // A confined pointer is held on its surface, wherever the motion asked it to go.
                let focus = confined_focus(&hold).or(focus);
                send_motion(&pointer, state, focus, location, time);
            }
            if let Some((target, event)) = relative {
                pointer.relative_motion(state, Some(target), &event);
            }
            pointer.frame(state);
        }
        PointerDelivery::Button {
            button,
            edge,
            serial,
            time,
        } => {
            pointer.button(
                state,
                &ButtonEvent {
                    serial: serial_to(serial),
                    time: time_of(time),
                    button: super::edge::button_of(button),
                    state: super::edge::button_state_of(edge),
                },
            );
            pointer.frame(state);
        }
    }
    Ok(())
}

/// Ends whatever grab holds the pointer (a popup's, once its popups are dismissed), restoring
/// the focus the pointer is over.
pub fn end_grab<D: SeatHandler + 'static>(
    seat: &Seat<D>,
    state: &mut D,
    serial: InputSerial,
    time: Mono,
) {
    if let Some(pointer) = seat.get_pointer() {
        pointer.unset_grab(state, serial_to(serial), time_of(time));
    }
}

/// A relative motion with the focus it goes to.
type RelativeDelivery<F> = ((F, Point<f64, Logical>), RelativeMotionEvent);

/// The relative motion for a delivery over `focus` (none when the pointer did not move or is
/// over nothing).
fn relative_of<F: From<WlSurface>>(
    focus: &Option<SurfaceAt>,
    delta: casement_core::geometry::SubPoint,
    time: InputTime,
) -> Option<RelativeDelivery<F>> {
    let at = focus.as_ref()?;
    if delta.x.0 == 0 && delta.y.0 == 0 {
        return None;
    }
    let moved = logical_of(delta);
    Some((
        (F::from(at.surface.clone()), whole_of(at.origin)),
        RelativeMotionEvent {
            delta: moved,
            delta_unaccel: moved,
            time,
        },
    ))
}

/// Recomputes pointer focus for a pointer that has not moved (A16, SP1 A1): `focus` is the
/// surface now under it, found by the caller from the scene. When it is the surface that
/// already has the pointer, its client hears a motion only if the surface has moved since it was
/// last told the position (the coordinates it holds are stale), and none while a constraint holds
/// the pointer; otherwise the old one gets `leave` and the new one `enter` at the pointer's own
/// position.
pub fn apply_refocus<D>(
    seat: &Seat<D>,
    state: &mut D,
    focus: Option<SurfaceAt>,
    time: Mono,
) -> Result<(), WaylandFault>
where
    D: SeatHandler + PointerConstraintsHandler + 'static,
    D::PointerFocus: From<WlSurface> + Into<Option<WlSurface>> + Clone + PartialEq,
{
    let pointer = seat.get_pointer().ok_or(WaylandFault::SurfaceGone)?;
    let current: Option<WlSurface> = pointer.current_focus().and_then(Into::into);
    let wanted = focus.as_ref().map(|at| &at.surface);
    if current.as_ref() == wanted {
        let stale = focus.as_ref().filter(|at| focus_origin::moved(at));
        let Some(at) = stale else {
            return Ok(());
        };
        if !matches!(hold_of(&pointer), Hold::Free) {
            return Ok(());
        }
        send_motion(
            &pointer,
            state,
            Some(at.clone()),
            pointer.current_location(),
            time_of(time),
        );
        pointer.frame(state);
        return Ok(());
    }
    let location = pointer.current_location();
    send_motion(&pointer, state, focus, location, time_of(time));
    pointer.frame(state);
    Ok(())
}

/// Delivers one scroll frame (`axis_source` and `axis_value120` always sent: SP1 B1).
pub fn apply_axis<D>(
    seat: &Seat<D>,
    state: &mut D,
    frame: AxisFrame,
    time: Mono,
) -> Result<(), WaylandFault>
where
    D: SeatHandler + 'static,
{
    let pointer = seat.get_pointer().ok_or(WaylandFault::SurfaceGone)?;
    pointer.axis(state, super::edge::axis_frame_of(frame, time));
    pointer.frame(state);
    Ok(())
}

fn cancelled(end: GestureEnd) -> bool {
    matches!(end, GestureEnd::Cancelled)
}

fn fingers_of(fingers: u8) -> u32 {
    u32::from(fingers)
}

/// Delivers a gesture step to the client under the pointer.
pub fn apply_gesture<D>(
    seat: &Seat<D>,
    state: &mut D,
    gesture: GestureDelivery,
    serial: InputSerial,
    time: Mono,
) -> Result<(), WaylandFault>
where
    D: SeatHandler + 'static,
{
    let pointer = seat.get_pointer().ok_or(WaylandFault::SurfaceGone)?;
    let (serial, time) = (serial_to(serial), time_of(time));
    match gesture {
        GestureDelivery::Swipe(GestureStep::Begin { fingers }) => pointer.gesture_swipe_begin(
            state,
            &GestureSwipeBeginEvent {
                serial,
                time,
                fingers: fingers_of(fingers),
            },
        ),
        GestureDelivery::Swipe(GestureStep::Update(delta)) => pointer.gesture_swipe_update(
            state,
            &GestureSwipeUpdateEvent {
                time,
                delta: logical_of(delta),
            },
        ),
        GestureDelivery::Swipe(GestureStep::End(end)) => pointer.gesture_swipe_end(
            state,
            &GestureSwipeEndEvent {
                serial,
                time,
                cancelled: cancelled(end),
            },
        ),
        GestureDelivery::Pinch(GestureStep::Begin { fingers }) => pointer.gesture_pinch_begin(
            state,
            &GesturePinchBeginEvent {
                serial,
                time,
                fingers: fingers_of(fingers),
            },
        ),
        GestureDelivery::Pinch(GestureStep::Update(update)) => {
            pointer.gesture_pinch_update(state, &pinch_update(update, time))
        }
        GestureDelivery::Pinch(GestureStep::End(end)) => pointer.gesture_pinch_end(
            state,
            &GesturePinchEndEvent {
                serial,
                time,
                cancelled: cancelled(end),
            },
        ),
        GestureDelivery::Hold(GestureStep::Begin { fingers }) => pointer.gesture_hold_begin(
            state,
            &GestureHoldBeginEvent {
                serial,
                time,
                fingers: fingers_of(fingers),
            },
        ),
        GestureDelivery::Hold(GestureStep::Update(())) => {}
        GestureDelivery::Hold(GestureStep::End(end)) => pointer.gesture_hold_end(
            state,
            &GestureHoldEndEvent {
                serial,
                time,
                cancelled: cancelled(end),
            },
        ),
    }
    Ok(())
}

/// A pinch update as smithay's: the scale is the model's thousandths, the rotation its
/// thousandths of a degree, the centre's movement in pixels.
fn pinch_update(update: PinchDelta, time: InputTime) -> GesturePinchUpdateEvent {
    GesturePinchUpdateEvent {
        time,
        delta: logical_of(update.delta),
        scale: f64::from(update.scale.0) / 1000.0,
        rotation: f64::from(update.rotation_millideg) / 1000.0,
    }
}
