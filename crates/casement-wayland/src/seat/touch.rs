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
use smithay::utils::{Logical, Point};

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
    match step {
        TouchStep::Down { slot, at } => touch.down(
            state,
            focus_of::<D>(focus),
            &DownEvent {
                slot: TouchSlot::from(Some(slot)),
                location: whole_of(at),
                serial: serial_to(serial),
                time,
            },
        ),
        TouchStep::Motion { slot, at } => touch.motion(
            state,
            focus_of::<D>(focus),
            &MotionEvent {
                slot: TouchSlot::from(Some(slot)),
                location: whole_of(at),
                time,
            },
        ),
        TouchStep::Up { slot } => touch.up(
            state,
            &UpEvent {
                slot: TouchSlot::from(Some(slot)),
                serial: serial_to(serial),
                time,
            },
        ),
        TouchStep::Frame => touch.frame(state),
        TouchStep::Cancel => touch.cancel(state),
    }
    Ok(())
}
