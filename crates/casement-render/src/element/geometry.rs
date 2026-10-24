//! Where a surface's xdg window geometry sits inside its tree, and so where the tree is drawn
//! and hit when the window's own origin is the geometry's.

use casement_core::geometry::{LogicalPoint, SubPoint, SubPx};
use casement_core::window::Margin;
use smithay::backend::renderer::utils::with_renderer_surface_state;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::utils::{Logical, Point, Size};
use smithay::wayland::compositor::with_states;
use smithay::wayland::shell::xdg::SurfaceCachedState;

/// Where `surface`'s committed window geometry starts inside it (zero without one: layer
/// surfaces, popups' roots, a toplevel that never called `set_window_geometry`).
pub(crate) fn geometry_origin(surface: &WlSurface) -> Point<i32, Logical> {
    with_states(surface, |states| {
        states
            .cached_state
            .get::<SurfaceCachedState>()
            .current()
            .geometry
            .map(|geometry| geometry.loc)
            .unwrap_or_default()
    })
}

/// The size of `surface`'s committed window geometry, if it declared one.
pub fn geometry_size(surface: &WlSurface) -> Option<Size<i32, Logical>> {
    with_states(surface, |states| {
        states
            .cached_state
            .get::<SurfaceCachedState>()
            .current()
            .geometry
            .map(|geometry| geometry.size)
    })
}

/// Where `root`'s top-left is when its window geometry's top-left is at `at`: the buffer is
/// drawn offset so the geometry origin lands on the window's place.
pub fn surface_origin(root: &WlSurface, at: SubPoint) -> SubPoint {
    let loc = geometry_origin(root);
    SubPoint {
        x: SubPx(at.x.0.saturating_sub(SubPx::of_logical(loc.x).0)),
        y: SubPx(at.y.0.saturating_sub(SubPx::of_logical(loc.y).0)),
    }
}

/// [`surface_origin`] for whole logical pixels (the hit test's).
pub fn surface_origin_logical(root: &WlSurface, at: LogicalPoint) -> LogicalPoint {
    let loc = geometry_origin(root);
    LogicalPoint {
        x: at.x.saturating_sub(loc.x),
        y: at.y.saturating_sub(loc.y),
    }
}

/// Whether `root`'s buffer reaches past its window geometry: a client that drew a shadow, or
/// anything else, round the window. A window with no geometry, or none committed yet, has none.
pub fn margin_of(root: &WlSurface) -> Margin {
    let buffer = with_renderer_surface_state(root, |state| state.surface_size()).flatten();
    match (buffer, geometry_size(root)) {
        (Some(buffer), Some(geometry)) if geometry.w < buffer.w || geometry.h < buffer.h => {
            Margin::Drawn
        }
        _ => Margin::None,
    }
}
