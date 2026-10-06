//! Where a surface's top-left was when its client was last given the pointer's coordinates.
//!
//! A client is told the pointer's position relative to its surface. When the surface moves
//! under a pointer that does not (a window dragged or placed, a layer rearranged), those
//! coordinates go stale unless the client hears a motion again; `apply_refocus` asks here
//! whether the origin changed since the last motion, and `send_motion` records each one.

use super::SurfaceAt;
use casement_core::geometry::LogicalPoint;
use smithay::wayland::compositor::with_states;
use std::sync::Mutex;

/// The origin the client was last told the pointer against, kept in the surface's data map.
#[derive(Debug, Default)]
struct Told(Mutex<Option<LogicalPoint>>);

/// Records that `focus`'s client has been given the pointer against `focus.origin`.
pub(super) fn told(focus: &SurfaceAt) {
    with_states(&focus.surface, |states| {
        let held = states.data_map.get_or_insert_threadsafe(Told::default);
        if let Ok(mut origin) = held.0.lock() {
            *origin = Some(focus.origin);
        }
    });
}

/// Whether `focus`'s surface has moved since its client was last told the pointer's position.
pub(super) fn moved(focus: &SurfaceAt) -> bool {
    with_states(&focus.surface, |states| {
        states
            .data_map
            .get::<Told>()
            .and_then(|held| held.0.lock().ok().map(|origin| *origin))
            .is_some_and(|origin| origin != Some(focus.origin))
    })
}

/// Where `surface`'s top-left was when its client was last given the pointer's position.
pub(super) fn of(
    surface: &smithay::reexports::wayland_server::protocol::wl_surface::WlSurface,
) -> Option<LogicalPoint> {
    with_states(surface, |states| {
        states
            .data_map
            .get::<Told>()
            .and_then(|held| held.0.lock().ok().and_then(|origin| *origin))
    })
}
