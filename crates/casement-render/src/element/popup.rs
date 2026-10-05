//! The popups of a window or layer surface as elements.

use super::CasementElement;
use crate::compositor::Compositor;
use crate::scene::OutputView;
use crate::scene::convert::{on_output, sub_of};
use casement_core::geometry::LogicalPoint;
use smithay::backend::renderer::element::Kind;
use smithay::backend::renderer::element::surface::render_elements_from_surface_tree;
use smithay::desktop::PopupManager;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::utils::{Logical, Point};
use smithay::wayland::compositor::with_states;
use smithay::wayland::shell::xdg::SurfaceCachedState;

/// Every popup surface whose root is `root` (smithay's popup tree), front first (a popup's
/// children before it, later popups before earlier ones), each with where its top-left is in
/// global logical pixels when `root`'s top-left sits at `origin`. The renderer draws popups
/// there and the pointer's hit test finds them there.
pub fn popup_surfaces(root: &WlSurface, origin: LogicalPoint) -> Vec<(WlSurface, LogicalPoint)> {
    let root_geometry = window_geometry_origin(root);
    let popups: Vec<_> = PopupManager::popups_for_surface(root).collect();
    popups
        .into_iter()
        .rev()
        .map(|(popup, offset)| {
            // The popup's surface origin: the root's window geometry, the popup's place in
            // it, less the popup's own geometry offset (smithay's popup tree convention).
            let at = root_geometry + offset - popup.geometry().loc;
            let corner = LogicalPoint {
                x: origin.x.saturating_add(at.x),
                y: origin.y.saturating_add(at.y),
            };
            (popup.wl_surface().clone(), corner)
        })
        .collect()
}

/// Every popup whose root is `root`, as elements at the places [`popup_surfaces`] gives, front
/// first.
pub fn popup_elements<R: Compositor>(
    root: &WlSurface,
    origin: LogicalPoint,
    view: &OutputView,
    renderer: &mut R,
) -> Vec<CasementElement<R>> {
    popup_surfaces(root, origin)
        .into_iter()
        .flat_map(|(surface, corner)| {
            render_elements_from_surface_tree(
                renderer,
                &surface,
                on_output(sub_of(corner), view),
                view.scale.as_f64(),
                1.0,
                Kind::Unspecified,
            )
        })
        .collect()
}

/// Where a surface's window geometry starts inside it (zero without one: layer surfaces).
fn window_geometry_origin(surface: &WlSurface) -> Point<i32, Logical> {
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
