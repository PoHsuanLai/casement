//! Pointer motion, buttons, scroll and gestures onto the wire, with pointer constraints applied
//! on the way (a locked pointer stays put and only relative motion arrives; a confined one does
//! not leave its region).

use super::edge::{logical_of, time_of, whole_of};
use super::focus_origin;
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
use smithay::wayland::compositor::RegionAttributes;
use smithay::wayland::pointer_constraints::{
    PointerConstraint, PointerConstraintsHandler, with_pointer_constraint,
};

/// What an active constraint does to a motion over its surface.
#[derive(Debug)]
enum Hold<S> {
    /// Nothing holds the pointer.
    Free,
    /// The pointer stays where it is.
    Locked(S),
    /// The pointer stays inside the region (surface-local; `None` is the whole surface).
    Confined(S, Option<RegionAttributes>),
}

/// The constraint that is active on the surface the pointer is over.
fn hold_of<D>(pointer: &PointerHandle<D>) -> Hold<WlSurface>
where
    D: SeatHandler + 'static,
    D::PointerFocus: Into<Option<WlSurface>>,
{
    let Some(surface) = pointer.current_focus().and_then(Into::into) else {
        return Hold::Free;
    };
    with_pointer_constraint::<D, _, _>(&surface, pointer, |constraint| match constraint {
        Some(held) if held.is_active() => match &*held {
            PointerConstraint::Locked(_) => Hold::Locked(surface.clone()),
            PointerConstraint::Confined(_) => {
                Hold::Confined(surface.clone(), held.region().cloned())
            }
        },
        Some(_) | None => Hold::Free,
    })
}

/// Whether `at` (global) is inside `region` of the surface whose origin is `origin`.
fn inside(
    region: Option<&RegionAttributes>,
    origin: Point<f64, Logical>,
    at: Point<f64, Logical>,
) -> bool {
    let local = at - origin;
    region.is_none_or(|region| region.contains(local.to_i32_floor()))
}

/// Where a motion to `wanted` over `over` (a surface and its origin) ends up under `hold`:
/// `None` when the pointer may not move at all (locked, or confined and the motion would leave
/// the region or the surface), else `wanted`.
fn settled<S: PartialEq>(
    hold: &Hold<S>,
    over: Option<(&S, Point<f64, Logical>)>,
    wanted: Point<f64, Logical>,
) -> Option<Point<f64, Logical>> {
    let origin_over = |held: &S| {
        over.filter(|(surface, _)| *surface == held)
            .map(|(_, at)| at)
    };
    match hold {
        Hold::Free => Some(wanted),
        Hold::Locked(held) => match origin_over(held) {
            Some(_) => None,
            None => Some(wanted),
        },
        Hold::Confined(held, region) => match origin_over(held) {
            Some(origin) if inside(region.as_ref(), origin, wanted) => Some(wanted),
            Some(_) | None => None,
        },
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use smithay::utils::Rectangle;
    use smithay::wayland::compositor::RectangleKind;

    fn at(x: f64, y: f64) -> Point<f64, Logical> {
        Point::from((x, y))
    }

    fn region(width: i32, height: i32) -> RegionAttributes {
        RegionAttributes {
            rects: vec![(
                RectangleKind::Add,
                Rectangle::new((0, 0).into(), (width, height).into()),
            )],
        }
    }

    const HERE: u32 = 7;
    const ELSEWHERE: u32 = 8;

    #[test]
    fn a_free_pointer_goes_where_it_is_sent() {
        let moved = settled(&Hold::Free, Some((&HERE, at(0.0, 0.0))), at(5.0, 5.0));
        assert_eq!(moved, Some(at(5.0, 5.0)));
    }

    #[test]
    fn a_locked_pointer_stays_put_over_its_surface_and_moves_off_it() {
        let hold = Hold::Locked(HERE);
        let over_it = settled(&hold, Some((&HERE, at(0.0, 0.0))), at(5.0, 5.0));
        assert_eq!(over_it, None);
        let off_it = settled(&hold, Some((&ELSEWHERE, at(0.0, 0.0))), at(5.0, 5.0));
        assert_eq!(off_it, Some(at(5.0, 5.0)));
        assert_eq!(settled(&hold, None, at(5.0, 5.0)), Some(at(5.0, 5.0)));
    }

    #[test]
    fn a_confined_pointer_does_not_leave_its_region_or_its_surface() {
        let hold = Hold::Confined(HERE, Some(region(100, 100)));
        let origin = at(10.0, 10.0);
        // (50, 50) is (40, 40) inside the region; (200, 200) and (9, 50) are outside it.
        let inside = settled(&hold, Some((&HERE, origin)), at(50.0, 50.0));
        assert_eq!(inside, Some(at(50.0, 50.0)));
        let beyond = settled(&hold, Some((&HERE, origin)), at(200.0, 200.0));
        assert_eq!(beyond, None);
        let left_of_it = settled(&hold, Some((&HERE, origin)), at(9.0, 50.0));
        assert_eq!(
            left_of_it, None,
            "left of the surface is outside the region"
        );
        let off_the_surface = settled(&hold, Some((&ELSEWHERE, origin)), at(50.0, 50.0));
        assert_eq!(
            off_the_surface, None,
            "another surface does not take a confined pointer"
        );
        assert_eq!(settled(&hold, None, at(50.0, 50.0)), None);
    }

    #[test]
    fn a_confinement_without_a_region_covers_the_whole_surface() {
        let hold = Hold::Confined(HERE, None);
        let moved = settled(&hold, Some((&HERE, at(0.0, 0.0))), at(900.0, 900.0));
        assert_eq!(moved, Some(at(900.0, 900.0)));
    }
}
