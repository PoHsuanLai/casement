//! `zwp_idle_inhibit_manager_v1`: a visible surface (a video) keeps the session awake.

use crate::states::GlobalFilter;
use casement_core::Event;
use smithay::reexports::wayland_protocols::wp::idle_inhibit::zv1::server::zwp_idle_inhibit_manager_v1::ZwpIdleInhibitManagerV1;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::{DisplayHandle, GlobalDispatch};
use smithay::wayland::GlobalData;
use smithay::wayland::idle_inhibit::{IdleInhibitHandler, IdleInhibitManagerState};

/// `zwp_idle_inhibit_manager_v1`.
#[derive(Debug)]
pub struct IdleInhibitPart {
    /// smithay's state.
    pub state: IdleInhibitManagerState,
}

impl IdleInhibitPart {
    /// Creates the global (version 1). Every connection sees it.
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> IdleInhibitPart
    where
        D: GlobalDispatch<ZwpIdleInhibitManagerV1, GlobalData> + IdleInhibitHandler + 'static,
    {
        let _ = role_filter;
        IdleInhibitPart {
            state: IdleInhibitManagerState::new::<D>(display),
        }
    }
}

/// `surface` started inhibiting idleness (it counts while visible).
pub fn on_inhibit(part: &mut IdleInhibitPart, surface: &WlSurface) -> Vec<Event> {
    let _ = (part, surface);
    todo!("P0-H8: idle_inhibit::on_inhibit")
}

/// `surface` stopped inhibiting.
pub fn on_uninhibit(part: &mut IdleInhibitPart, surface: &WlSurface) -> Vec<Event> {
    let _ = (part, surface);
    todo!("P0-H8: idle_inhibit::on_uninhibit")
}
