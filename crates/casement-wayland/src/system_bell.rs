//! `xdg_system_bell_v1`: the alert sound, attributed to an app when a surface is given.

use crate::states::GlobalFilter;
use casement_core::Event;
use smithay::reexports::wayland_protocols::xdg::system_bell::v1::server::xdg_system_bell_v1::XdgSystemBellV1;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::{DisplayHandle, GlobalDispatch};
use smithay::wayland::GlobalData;
use smithay::wayland::xdg_system_bell::{XdgSystemBellHandler, XdgSystemBellState};

/// `xdg_system_bell_v1`.
#[derive(Debug)]
pub struct SystemBellPart {
    /// smithay's state.
    pub state: XdgSystemBellState,
}

impl SystemBellPart {
    /// Creates the global (version 1). Every connection sees it.
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> SystemBellPart
    where
        D: XdgSystemBellHandler + GlobalDispatch<XdgSystemBellV1, GlobalData>,
    {
        let _ = role_filter;
        SystemBellPart {
            state: XdgSystemBellState::new::<D>(display),
        }
    }
}

/// The bell rang, for `surface`'s app when given (`Effect::Bell` follows from the model).
pub fn on_bell(part: &mut SystemBellPart, surface: Option<&WlSurface>) -> Vec<Event> {
    let _ = (part, surface);
    todo!("P0-H2: system_bell::on_bell")
}
