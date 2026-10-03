//! `wl_data_device_manager` (P0-H5), and the selection and drag traits it rests on.
//!
//! The drag grab is started after anvil's `dnd_requested` (smithay, MIT); the pointer's focus
//! during the drag is whatever the seat's pointer delivery names, so a drag can end on a
//! layer surface exactly as it can on a window.

use crate::core_loop;
use crate::state::State;
use casement_wayland::convert::point_of;
use casement_wayland::data_device::{self, SelectionKind};
use casement_wayland::{ext_data_control, wlr_data_control};
use smithay::input::Seat;
use smithay::input::dnd::{DnDGrab, DndGrabHandler, DndTarget, GrabType, Source};
use smithay::input::pointer::Focus;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::utils::{Logical, Point, Serial};
use smithay::wayland::selection::data_device::{
    DataDeviceHandler, DataDeviceState, WaylandDndGrabHandler,
};
use smithay::wayland::selection::{SelectionHandler, SelectionSource, SelectionTarget};

impl SelectionHandler for State {
    type SelectionUserData = ();

    fn new_selection(
        &mut self,
        ty: SelectionTarget,
        source: Option<SelectionSource>,
        _seat: Seat<State>,
    ) {
        let kind = SelectionKind::of(ty);
        let mut events = data_device::on_selection(&mut self.wl.data_device, kind, source.as_ref());
        events.extend(ext_data_control::on_control_selection(
            &mut self.wl.ext_data_control,
            kind,
        ));
        events.extend(wlr_data_control::on_control_selection(
            &mut self.wl.wlr_data_control,
            kind,
        ));
        core_loop::feed(self, events);
    }
}

impl DataDeviceHandler for State {
    fn data_device_state(&mut self) -> &mut DataDeviceState {
        &mut self.wl.data_device.state
    }
}

impl WaylandDndGrabHandler for State {
    fn dnd_requested<S: Source>(
        &mut self,
        source: S,
        icon: Option<WlSurface>,
        seat: Seat<State>,
        serial: Serial,
        type_: GrabType,
    ) {
        let origin = seat
            .get_pointer()
            .and_then(|pointer| pointer.current_focus());
        let events =
            data_device::on_drag_start(&mut self.wl.data_device, origin.as_ref(), icon.as_ref());
        let display = self.display.clone();
        match type_ {
            GrabType::Pointer => {
                let Some(pointer) = seat.get_pointer() else {
                    source.cancel();
                    return;
                };
                let Some(start) = pointer.grab_start_data() else {
                    source.cancel();
                    return;
                };
                pointer.set_grab(
                    self,
                    DnDGrab::new_pointer(&display, start, source, seat),
                    serial,
                    Focus::Keep,
                );
            }
            GrabType::Touch => {
                let Some(touch) = seat.get_touch() else {
                    source.cancel();
                    return;
                };
                let Some(start) = touch.grab_start_data() else {
                    source.cancel();
                    return;
                };
                touch.set_grab(
                    self,
                    DnDGrab::new_touch(&display, start, source, seat),
                    serial,
                );
            }
        }
        core_loop::feed(self, events);
    }
}

impl DndGrabHandler for State {
    fn dropped(
        &mut self,
        _target: Option<DndTarget<'_, State>>,
        _validated: bool,
        _seat: Seat<State>,
        location: Point<f64, Logical>,
    ) {
        let events =
            data_device::on_drop(&mut self.wl.data_device, point_of(location.to_i32_round()));
        core_loop::feed(self, events);
    }

    fn cancelled(&mut self, _seat: Seat<State>, location: Point<f64, Logical>) {
        let events =
            data_device::on_drop(&mut self.wl.data_device, point_of(location.to_i32_round()));
        core_loop::feed(self, events);
    }
}
