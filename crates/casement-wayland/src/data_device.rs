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
use smithay::wayland::selection::data_device::set_data_device_focus;
use smithay::wayland::selection::data_device::{DataDeviceHandler, DataDeviceState};
use smithay::wayland::selection::primary_selection::{PrimarySelectionHandler, set_primary_focus};
use smithay::wayland::selection::{SelectionSource, SelectionTarget};
use std::collections::BTreeMap;

/// `wl_data_device_manager`.
#[derive(Debug)]
pub struct DataDevicePart {
    /// smithay's state.
    pub state: DataDeviceState,
    /// The mime types each selection offers now (empty when cleared).
    pub selections: BTreeMap<SelectionKind, Vec<String>>,
    /// The drag in progress, if any.
    pub drag: Drag,
}

/// Whether a drag is under way.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Drag {
    /// No drag.
    #[default]
    Idle,
    /// A client is dragging: where it started and the icon it asked to be drawn.
    Under {
        /// The surface the drag started from.
        origin: Option<WlSurface>,
        /// The icon surface, drawn at the pointer.
        icon: Option<WlSurface>,
    },
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
            selections: BTreeMap::new(),
            drag: Drag::Idle,
        }
    }
}

/// Which selection changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
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

/// A client set (or cleared) a selection. smithay offers it to the focused client and to every
/// clipboard manager; the part keeps the mime types it offers.
pub fn on_selection(
    part: &mut DataDevicePart,
    kind: SelectionKind,
    source: Option<&SelectionSource>,
) -> Vec<Event> {
    let offered = source.map(SelectionSource::mime_types).unwrap_or_default();
    part.selections.insert(kind, offered);
    Vec::new()
}

/// A client started a drag from `origin` with an optional icon surface.
pub fn on_drag_start(
    part: &mut DataDevicePart,
    origin: Option<&WlSurface>,
    icon: Option<&WlSurface>,
) -> Vec<Event> {
    part.drag = Drag::Under {
        origin: origin.cloned(),
        icon: icon.cloned(),
    };
    Vec::new()
}

/// A drag ended (dropped at `at`, global logical, or cancelled): the icon goes away.
pub fn on_drop(part: &mut DataDevicePart, at: LogicalPoint) -> Vec<Event> {
    let _ = at;
    part.drag = Drag::Idle;
    Vec::new()
}

/// Gives the clipboard and primary-selection focus to `client` (the keyboard focus's client,
/// or nobody).
pub fn apply_selection_focus<D>(
    display: &DisplayHandle,
    seat: &Seat<D>,
    client: Option<Client>,
) -> Result<(), WaylandFault>
where
    D: SeatHandler + DataDeviceHandler + PrimarySelectionHandler + 'static,
{
    set_data_device_focus(display, seat, client.clone());
    set_primary_focus(display, seat, client);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_kinds_follow_smithays_targets() {
        assert_eq!(
            SelectionKind::of(SelectionTarget::Clipboard),
            SelectionKind::Clipboard
        );
        assert_eq!(
            SelectionKind::of(SelectionTarget::Primary),
            SelectionKind::Primary
        );
    }
}
