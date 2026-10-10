//! `zwlr_foreign_toplevel_manager_v1`: the window list for docks and switchers that speak the
//! wlr protocol (sill's `desktop_wl` Wlr arm). P1-Z declares it; P1-M2 creates the global and
//! fills the functions. Every window is listed with its title, app id, state and the outputs
//! it is on; the handle's requests become `Event::WlrToplevel`.

use crate::states::{GlobalFilter, WaylandFault};
use casement_core::window::{Window, WlrToplevelRequest};
use casement_core::{Event, OutputName, WindowId};
use smithay::reexports::wayland_server::DisplayHandle;
use std::collections::BTreeSet;

/// `zwlr_foreign_toplevel_manager_v1` and the windows listed on it.
#[derive(Debug, Default)]
pub struct WlrToplevelPart {
    /// Each window with a handle.
    pub listed: BTreeSet<WindowId>,
}

impl WlrToplevelPart {
    /// Creates the global (version 3), filtered by the I7 table. P1-Z: creates nothing.
    pub fn new<D: 'static>(display: &DisplayHandle, role_filter: GlobalFilter) -> WlrToplevelPart {
        let _ = (display, role_filter, std::marker::PhantomData::<D>);
        WlrToplevelPart::default()
    }
}

/// A request on `window`'s handle.
///
/// ```no_run
/// use casement_core::WindowId;
/// use casement_core::window::WlrToplevelRequest;
/// use casement_wayland::wlr_toplevel::{WlrToplevelPart, on_request};
///
/// let mut part = WlrToplevelPart::default();
/// let events = on_request(&mut part, WindowId(1), WlrToplevelRequest::Close);
/// assert_eq!(events.len(), 1);
/// ```
pub fn on_request(
    part: &mut WlrToplevelPart,
    window: WindowId,
    request: WlrToplevelRequest,
) -> Vec<Event> {
    let _ = (part, window, request);
    todo!("P1-M2: wlr_toplevel::on_request")
}

/// Lists `window` (or sends what changed: title, app id, state, the outputs it is on), then
/// `done`. `outputs` are the connectors it overlaps.
pub fn apply_toplevel(
    part: &mut WlrToplevelPart,
    window: &Window,
    outputs: &[OutputName],
) -> Result<(), WaylandFault> {
    let _ = (part, window, outputs);
    todo!("P1-M2: wlr_toplevel::apply_toplevel")
}

/// Withdraws `window`: its handles get `closed`.
pub fn apply_closed(part: &mut WlrToplevelPart, window: WindowId) -> Result<(), WaylandFault> {
    let _ = (part, window);
    todo!("P1-M2: wlr_toplevel::apply_closed")
}
