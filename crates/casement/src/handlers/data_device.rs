//! `wl_data_device_manager` (P0-H5), and the selection and drag traits it rests on.

use crate::state::State;
use smithay::input::dnd::DndGrabHandler;
use smithay::wayland::selection::SelectionHandler;
use smithay::wayland::selection::data_device::{
    DataDeviceHandler, DataDeviceState, WaylandDndGrabHandler,
};

impl SelectionHandler for State {
    type SelectionUserData = ();
}

impl DataDeviceHandler for State {
    fn data_device_state(&mut self) -> &mut DataDeviceState {
        &mut self.wl.data_device.state
    }
}

impl WaylandDndGrabHandler for State {}

impl DndGrabHandler for State {}
