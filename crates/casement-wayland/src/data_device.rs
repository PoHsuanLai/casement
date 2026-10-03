//! `wl_data_device_manager`: the clipboard and drag and drop. Selection focus follows
//! keyboard focus; a drag can end on a layer surface.

use crate::states::{GlobalFilter, WaylandFault};
use casement_core::Event;
use casement_core::geometry::LogicalPoint;
use smithay::input::{Seat, SeatHandler};
use smithay::reexports::wayland_server::protocol::wl_data_device_manager::WlDataDeviceManager;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::{Client, DisplayHandle, GlobalDispatch};
use smithay::wayland::GlobalData;
use smithay::wayland::selection::data_device::{DataDeviceHandler, DataDeviceState};
use smithay::wayland::selection::{SelectionSource, SelectionTarget};

/// `wl_data_device_manager`.
#[derive(Debug)]
pub struct DataDevicePart {
    /// smithay's state.
    pub state: DataDeviceState,
}

impl DataDevicePart {
    /// Creates the global (version 3). Every connection sees it.
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> DataDevicePart
    where
        D: GlobalDispatch<WlDataDeviceManager, GlobalData> + DataDeviceHandler + 'static,
    {
        let _ = role_filter;
        DataDevicePart {
            state: DataDeviceState::new::<D>(display),
        }
    }
}

/// Which selection changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SelectionKind {
    /// The clipboard.
    Clipboard,
    /// The primary selection (middle click).
    Primary,
}

impl SelectionKind {
    /// smithay's selection target as ours.
    pub fn of(target: SelectionTarget) -> SelectionKind {
        match target {
            SelectionTarget::Clipboard => SelectionKind::Clipboard,
            SelectionTarget::Primary => SelectionKind::Primary,
        }
    }
}

/// A client set (or cleared) a selection.
pub fn on_selection(
    part: &mut DataDevicePart,
    kind: SelectionKind,
    source: Option<&SelectionSource>,
) -> Vec<Event> {
    let _ = (part, kind, source);
    todo!("P0-H5: data_device::on_selection")
}

/// A client started a drag from `origin` with an optional icon surface.
pub fn on_drag_start(
    part: &mut DataDevicePart,
    origin: Option<&WlSurface>,
    icon: Option<&WlSurface>,
) -> Vec<Event> {
    let _ = (part, origin, icon);
    todo!("P0-H5: data_device::on_drag_start")
}

/// A drag was dropped at `at` (global logical).
pub fn on_drop(part: &mut DataDevicePart, at: LogicalPoint) -> Vec<Event> {
    let _ = (part, at);
    todo!("P0-H5: data_device::on_drop")
}

/// Gives the clipboard and primary-selection focus to `client` (the keyboard focus's client,
/// or nobody).
pub fn apply_selection_focus<D>(
    display: &DisplayHandle,
    seat: &Seat<D>,
    client: Option<Client>,
) -> Result<(), WaylandFault>
where
    D: SeatHandler + DataDeviceHandler + 'static,
{
    let _ = (display, seat, client);
    todo!("P0-H5: data_device::apply_selection_focus")
}
