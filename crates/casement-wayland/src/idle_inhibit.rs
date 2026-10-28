//! `zwp_idle_inhibit_manager_v1`: a visible surface (a video) keeps the session awake.

use crate::states::GlobalFilter;
use smithay::reexports::wayland_protocols::wp::idle_inhibit::zv1::server::zwp_idle_inhibit_manager_v1::ZwpIdleInhibitManagerV1;
use smithay::reexports::wayland_server::Resource;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::{Client, DisplayHandle, GlobalDispatch};
use smithay::wayland::GlobalData;
use smithay::wayland::idle_inhibit::{IdleInhibitHandler, IdleInhibitManagerState};

/// Inhibitors the session counts in all, whoever holds them: each is looked at whenever the
/// windows change, so the walk is bounded however many clients ask. One past it is not counted
/// (a player never loses its connection over another client's inhibitors).
pub const MAX_INHIBITORS: usize = 128;

/// `zwp_idle_inhibit_manager_v1` and the surfaces that hold idleness off (one entry per live
/// inhibitor, so a surface with two counts twice).
#[derive(Debug)]
pub struct IdleInhibitPart {
    /// smithay's state.
    pub state: IdleInhibitManagerState,
    inhibitors: Vec<WlSurface>,
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
            inhibitors: Vec::new(),
        }
    }

    /// How many inhibitors `client` holds.
    pub fn held_by(&self, client: &Client) -> usize {
        self.inhibitors
            .iter()
            .filter(|surface| {
                surface
                    .client()
                    .is_some_and(|owner| owner.id() == client.id())
            })
            .count()
    }

    /// How many inhibitors are held.
    pub fn len(&self) -> usize {
        self.inhibitors.len()
    }

    /// Whether none is held (a loop turn with none has nothing to look at).
    pub fn is_empty(&self) -> bool {
        self.inhibitors.is_empty()
    }

    /// Drops the inhibitors whose surface is gone (their client hung up or destroyed the
    /// surface): smithay calls `uninhibit` only for an explicit destroy, so without this a
    /// crashed player's entry would stay for the rest of the session.
    pub fn forget_dead(&mut self) {
        self.inhibitors.retain(WlSurface::is_alive);
    }

    /// Whether any inhibitor's surface still exists. smithay never reports an inhibitor whose
    /// client left, so a dead surface stops counting here.
    ///
    /// Limit: a protocol says an inhibitor counts only while its surface is visible, which
    /// needs the surface's window and the shown Spaces; neither lookup exists yet (xdg_shell
    /// keeps the window of each toplevel), so a live surface counts whether shown or not.
    /// [`IdleInhibitPart::inhibited_where`] takes the visibility test once there is one.
    pub fn inhibited(&self) -> bool {
        self.inhibited_where(|_| true)
    }

    /// Whether any inhibitor's live surface satisfies `visible`.
    pub fn inhibited_where(&self, visible: impl Fn(&WlSurface) -> bool) -> bool {
        self.inhibitors
            .iter()
            .any(|surface| surface.is_alive() && visible(surface))
    }
}

/// `surface` started inhibiting idleness (it counts while visible); says whether idleness is
/// now held off. The model has no idle event, so the answer goes to
/// `idle_notify::on_inhibit` instead of the core loop.
pub fn on_inhibit(part: &mut IdleInhibitPart, surface: &WlSurface) -> bool {
    part.forget_dead();
    if part.inhibitors.len() < MAX_INHIBITORS {
        part.inhibitors.push(surface.clone());
    } else {
        tracing::warn!("too many idle inhibitors are held; one is not counted");
    }
    part.inhibited()
}

/// `surface` stopped inhibiting (one inhibitor of it); says whether idleness is still held off.
pub fn on_uninhibit(part: &mut IdleInhibitPart, surface: &WlSurface) -> bool {
    if let Some(at) = part.inhibitors.iter().position(|held| held == surface) {
        part.inhibitors.remove(at);
    }
    part.forget_dead();
    part.inhibited()
}

// The session-wide cap is a bound on the walk; the per-client cap is `limits::Limit`'s.
const _: () = assert!(MAX_INHIBITORS > crate::limits::MAX_IDLE_INHIBITORS);
