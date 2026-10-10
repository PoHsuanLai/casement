//! A37: the wlcs-only relaxation of "must ack the initial configure before attaching a buffer".
//!
//! smithay refuses a buffer attached before the first `ack_configure` (a protocol error,
//! `xdg_surface.unconfigured_buffer`); wlcs' helpers at the pinned SHA attach first, which is
//! about 260 failures that say nothing about the compositor (SP5). With [`AckPolicy::Lenient`]
//! a commit hook, added when the `wl_surface` is created so it runs before smithay's own hooks
//! (added at `get_toplevel`, `get_popup` and `get_layer_surface`), records the server's current
//! state as acked where a buffer is pending and nothing has been acked: what the client would
//! have sent.
//!
//! Behind the `testing` feature, and off ([`AckPolicy::Strict`]) until a fixture asks for it
//! (`State::ack_policy`, set from `FixtureSpec::with_ack_policy`): only the wlcs shim and its
//! own test do, so every other test keeps the strict rule.

use crate::state::State;
use smithay::reexports::wayland_server::DisplayHandle;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::utils::Serial;
use smithay::wayland::compositor::{
    BufferAssignment, SurfaceAttributes, add_pre_commit_hook, with_states,
};
use smithay::wayland::shell::wlr_layer::{LayerSurfaceConfigure, LayerSurfaceData};
use smithay::wayland::shell::xdg::{
    PopupConfigure, ToplevelConfigure, XdgPopupSurfaceData, XdgToplevelSurfaceData,
};

/// What a commit that attaches a buffer before the first ack is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AckPolicy {
    /// A protocol error (the protocol's rule, and the default).
    #[default]
    Strict,
    /// Taken as the ack the client skipped (wlcs' helpers need it).
    Lenient,
}

/// Adds the relaxation to `surface`.
pub(crate) fn install(surface: &WlSurface) {
    add_pre_commit_hook::<State, _>(surface, pre_commit);
}

fn pre_commit(state: &mut State, _dh: &DisplayHandle, surface: &WlSurface) {
    if state.ack_policy == AckPolicy::Strict {
        return;
    }
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
        if let Some(role) = states.data_map.get::<LayerSurfaceData>()
            && let Ok(mut role) = role.lock()
            && role.last_acked.is_none()
        {
            let state = role.server_pending.unwrap_or_default();
            role.last_acked = Some(LayerSurfaceConfigure {
                state,
                serial: latest_serial(None),
            });
        }
    });
}

/// The serial of the newest configure sent, or zero when none was.
fn latest_serial(newest: Option<Serial>) -> Serial {
    newest.unwrap_or_else(|| Serial::from(0))
}
