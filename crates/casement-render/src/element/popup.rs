//! The popups of a window or layer surface as elements.

use super::CasementElement;
use super::geometry::geometry_origin;
use crate::compositor::Compositor;
use crate::scene::OutputView;
use crate::scene::convert::{on_output, sub_of};
use casement_core::geometry::LogicalPoint;
use smithay::backend::renderer::element::Kind;
use smithay::backend::renderer::element::surface::{
    WaylandSurfaceRenderElement, render_elements_from_surface_tree,
};
use smithay::backend::renderer::element::utils::RescaleRenderElement;
use smithay::desktop::PopupManager;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::utils::Scale;

/// Every popup surface whose root is `root` (smithay's popup tree), front first (a popup's
/// children before it, later popups before earlier ones), each with where its top-left is in
/// global logical pixels when `root`'s top-left sits at `origin`. The renderer draws popups
/// there and the pointer's hit test finds them there.
pub fn popup_surfaces(root: &WlSurface, origin: LogicalPoint) -> Vec<(WlSurface, LogicalPoint)> {
    let root_geometry = geometry_origin(root);
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

/// How a window's popups ride with it through a Scale flight: stretched by `ratio` about
/// `about` (the window's top-left) and drawn at `alpha`, the window's opacity.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct PopupRide {
    /// The point the popups are stretched about, global logical pixels.
    pub about: LogicalPoint,
    /// The stretch, per axis.
    pub ratio: (f64, f64),
    /// The opacity.
    pub alpha: f32,
}

impl PopupRide {
    /// Popups stretched by `ratio` about `about` and drawn at `alpha`.
    pub fn new(about: LogicalPoint, ratio: (f64, f64), alpha: f32) -> PopupRide {
        PopupRide {
            about,
            ratio,
            alpha,
        }
    }
}

/// [`popup_elements`] for a window in a Scale flight: each popup is laid out at its real size
/// and stretched by the ratio about `ride.about`, so a menu shrinks toward its window's corner
/// with it and fades with it.
pub fn popup_elements_riding<R: Compositor>(
    root: &WlSurface,
    origin: LogicalPoint,
    ride: PopupRide,
    view: &OutputView,
    renderer: &mut R,
) -> Vec<CasementElement<R>> {
    let about = on_output(sub_of(ride.about), view);
    popup_surfaces(root, origin)
        .into_iter()
        .flat_map(|(surface, corner)| {
            render_elements_from_surface_tree::<R, WaylandSurfaceRenderElement<R>>(
                renderer,
                &surface,
                on_output(sub_of(corner), view),
                view.scale.as_f64(),
                ride.alpha,
                Kind::Unspecified,
            )
        })
        .map(|popup| {
            CasementElement::Stretched(RescaleRenderElement::from_element(
                popup,
                about,
                Scale::from(ride.ratio),
            ))
        })
        .collect()
}
