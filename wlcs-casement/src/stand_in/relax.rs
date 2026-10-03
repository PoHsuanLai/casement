//! A37: the wlcs-only relaxation of "must ack the initial configure before attaching a buffer".
//!
//! smithay refuses a buffer attached before the first `ack_configure` (a protocol error,
//! `xdg_surface.unconfigured_buffer`); wlcs' helpers at the pinned SHA attach first, which is
//! about 260 failures that say nothing about the compositor (SP5). The relaxation is a commit
//! hook added when the `wl_surface` is created, so it runs before smithay's own xdg hook (added
//! at `get_toplevel` / `get_popup`): where a buffer is pending and nothing has been acked, it
//! records the server's current state as acked, which is what the client would have sent.
//!
//! Behind the `testing` feature only. The same hook goes into casement's compositor handler
//! behind casement's `testing` feature (an interface ask: it is H1/H2's file).

use super::StandIn;
use smithay::reexports::wayland_server::DisplayHandle;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::utils::Serial;
use smithay::wayland::compositor::{
    BufferAssignment, SurfaceAttributes, add_pre_commit_hook, with_states,
};
use smithay::wayland::shell::xdg::{
    PopupConfigure, ToplevelConfigure, XdgPopupSurfaceData, XdgToplevelSurfaceData,
};

/// Adds the relaxation to `surface`.
pub fn install(surface: &WlSurface) {
    add_pre_commit_hook::<StandIn, _>(surface, pre_commit);
}

fn pre_commit(_state: &mut StandIn, _dh: &DisplayHandle, surface: &WlSurface) {
    with_states(surface, |states| {
        let attaches = matches!(
            states
                .cached_state
                .get::<SurfaceAttributes>()
                .pending()
                .buffer,
            Some(BufferAssignment::NewBuffer(_))
        );
        if !attaches {
            return;
        }
        if let Some(role) = states.data_map.get::<XdgToplevelSurfaceData>()
            && let Ok(mut role) = role.lock()
            && role.last_acked.is_none()
        {
            let state = role.current_server_state();
            let serial = latest_serial(role.pending_configures().last().map(|c| c.serial));
            role.last_acked = Some(ToplevelConfigure { state, serial });
        }
        if let Some(role) = states.data_map.get::<XdgPopupSurfaceData>()
            && let Ok(mut role) = role.lock()
            && role.last_acked.is_none()
        {
            let state = role.current_server_state();
            let serial = latest_serial(role.pending_configures().last().map(|c| c.serial));
            role.last_acked = Some(PopupConfigure {
                state,
                serial,
                reposition_token: None,
            });
        }
    });
}

/// The serial of the newest configure sent, or zero when none was.
fn latest_serial(newest: Option<Serial>) -> Serial {
    newest.unwrap_or_else(|| Serial::from(0))
}
