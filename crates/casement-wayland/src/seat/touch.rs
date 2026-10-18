//! Touch steps onto the wire.

use super::SurfaceAt;
use super::edge::{time_of, whole_of};
use crate::convert::serial_to;
use crate::states::WaylandFault;
use casement_core::InputSerial;
use casement_core::input::TouchStep;
use casement_core::time::Mono;
use smithay::backend::input::TouchSlot;
use smithay::input::touch::{DownEvent, MotionEvent, UpEvent};
use smithay::input::{Seat, SeatHandler};
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::{Client, Resource};
use smithay::utils::{Logical, Point, SERIAL_COUNTER};
use std::collections::BTreeMap;
use std::sync::Mutex;

/// The surface each touch point that is down went down on, kept by the seat so that a point
/// can be released when its surface dies (smithay keeps the focus but offers no way to list it).
#[derive(Debug, Default)]
struct Points(Mutex<BTreeMap<u32, Down>>);

/// The surface a point went down on and the client that owned it (a dead surface names none).
#[derive(Debug)]
struct Down {
    surface: WlSurface,
    client: Option<Client>,
}

impl Points {
    fn of<D: SeatHandler + 'static>(seat: &Seat<D>) -> &Points {
        seat.user_data()
            .insert_if_missing_threadsafe(Points::default);
        seat.user_data()
            .get::<Points>()
            .unwrap_or_else(|| unreachable!("the touch points were inserted on the line above"))
    }

    fn with<R>(&self, f: impl FnOnce(&mut BTreeMap<u32, Down>) -> R) -> R {
        f(&mut self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner))
    }
}

/// The focus smithay's touch handle takes: the surface and where its origin is.
fn focus_of<D: SeatHandler>(
    focus: Option<SurfaceAt>,
) -> Option<(D::TouchFocus, Point<f64, Logical>)>
where
    D::TouchFocus: From<smithay::reexports::wayland_server::protocol::wl_surface::WlSurface>,
{
    focus.map(|at| (D::TouchFocus::from(at.surface), whole_of(at.origin)))
}

/// Delivers a touch step over `focus` (the surface a touch point went down on; motion keeps
/// the focus its `down` set).
pub fn apply_touch<D>(
    seat: &Seat<D>,
    state: &mut D,
    step: TouchStep,
    focus: Option<SurfaceAt>,
    serial: InputSerial,
    time: Mono,
) -> Result<(), WaylandFault>
where
    D: SeatHandler + 'static,
    D::TouchFocus: From<smithay::reexports::wayland_server::protocol::wl_surface::WlSurface>,
{
    let touch = seat.get_touch().ok_or(WaylandFault::SurfaceGone)?;
    let time = time_of(time);
    let points = Points::of(seat);
    match step {
        TouchStep::Down { slot, at } => {
            if let Some(at) = &focus {
                let down = Down {
                    surface: at.surface.clone(),
                    client: at.surface.client(),
                };
                points.with(|points| points.insert(slot, down));
            }
            touch.down(
                state,
                focus_of::<D>(focus),
                &DownEvent {
                    slot: TouchSlot::from(Some(slot)),
                    location: whole_of(at),
                    serial: serial_to(serial),
                    time,
                },
            )
        }
        TouchStep::Motion { slot, at } => touch.motion(
            state,
            focus_of::<D>(focus),
            &MotionEvent {
                slot: TouchSlot::from(Some(slot)),
                location: whole_of(at),
                time,
            },
        ),
        TouchStep::Up { slot } => {
            points.with(|down| down.remove(&slot));
            touch.up(
                state,
                &UpEvent {
                    slot: TouchSlot::from(Some(slot)),
                    serial: serial_to(serial),
                    time,
                },
            )
        }
        TouchStep::Frame => touch.frame(state),
        TouchStep::Cancel => {
            points.with(BTreeMap::clear);
            touch.cancel(state)
        }
    }
    Ok(())
}

/// A surface died: every touch point that is down on it is lifted, so its client (which still
/// has its `wl_touch`) is not left holding a point that never ends.
pub fn release_dead<D>(seat: &Seat<D>, state: &mut D, dead: &WlSurface, time: Mono)
where
    D: SeatHandler + 'static,
    D::TouchFocus: From<WlSurface>,
{
    let Some(touch) = seat.get_touch() else {
        return;
    };
    let lifted: Vec<(u32, Option<Client>)> = Points::of(seat).with(|down| {
        let slots: Vec<u32> = down
            .iter()
            .filter(|(_, point)| point.surface == *dead)
            .map(|(slot, _)| *slot)
            .collect();
        slots
            .into_iter()
            .filter_map(|slot| down.remove(&slot).map(|point| (slot, point.client)))
            .collect()
    });
    for (slot, client) in &lifted {
        let serial = SERIAL_COUNTER.next_serial();
        let event = UpEvent {
            slot: TouchSlot::from(Some(*slot)),
            serial,
            time: time_of(time),
        };
        // smithay clears the point's focus; it sends nothing to a surface that is gone, so the
        // client's own `wl_touch` is told directly.
        touch.up(state, &event);
        for wl_touch in client.iter().flat_map(|client| touch.client_touch(client)) {
            wl_touch.up(serial.into(), event.time.millis(), i32::from(event.slot));
            wl_touch.frame();
        }
    }
    if !lifted.is_empty() {
        touch.frame(state);
    }
}
