//! The popups of a window or layer surface as elements.

use super::CasementElement;
use super::geometry::geometry_origin;
use crate::compositor::Compositor;
use crate::scene::OutputView;
use crate::scene::convert::{on_output, sub_of};
use casement_core::geometry::{LogicalPoint, SubPoint, SubPx};
use smithay::backend::renderer::element::Kind;
use smithay::backend::renderer::element::surface::render_elements_from_surface_tree;
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

/// [`popup_elements`] for a window in a Scale flight: each popup's corner moves toward `ride.about`
/// by the ratio and the popup is drawn stretched by it, so a menu shrinks with its window and
/// fades with it.
pub fn popup_elements_riding<R: Compositor>(
    root: &WlSurface,
    origin: LogicalPoint,
    ride: PopupRide,
    view: &OutputView,
    renderer: &mut R,
) -> Vec<CasementElement<R>> {
    let scale = view.scale.as_f64();
    popup_surfaces(root, origin)
        .into_iter()
        .flat_map(|(surface, corner)| {
            render_elements_from_surface_tree(
                renderer,
                &surface,
                on_output(ridden_corner(corner, &ride), view),
                Scale::from((scale * ride.ratio.0, scale * ride.ratio.1)),
                ride.alpha,
                Kind::Unspecified,
            )
        })
        .collect()
}

/// Where a popup whose top-left is `corner` is when its window is stretched as `ride` says
/// (global sub-pixels).
pub(crate) fn ridden_corner(corner: LogicalPoint, ride: &PopupRide) -> SubPoint {
    let axis = |at: i32, about: i32, by: f64| {
        let from = f64::from(SubPx::of_logical(about).0);
        let apart = f64::from(SubPx::of_logical(at.saturating_sub(about)).0);
        SubPx((from + apart * by).round() as i32)
    };
    SubPoint {
        x: axis(corner.x, ride.about.x, ride.ratio.0),
        y: axis(corner.y, ride.about.y, ride.ratio.1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_popup_moves_toward_the_window_corner_by_the_ratio() {
        let about = LogicalPoint { x: 100, y: 200 };
        // name, corner, ratio, sub-pixels (120 per pixel)
        let cases = [
            ("unchanged", (140, 260), (1.0, 1.0), (140 * 120, 260 * 120)),
            ("half", (140, 260), (0.5, 0.5), (120 * 120, 230 * 120)),
            ("one axis", (140, 260), (1.0, 0.5), (140 * 120, 230 * 120)),
            (
                "at the corner",
                (100, 200),
                (0.1, 0.1),
                (100 * 120, 200 * 120),
            ),
            ("left of it", (60, 200), (0.5, 1.0), (80 * 120, 200 * 120)),
        ];
        for (name, corner, ratio, want) in cases {
            let ride = PopupRide::new(about, ratio, 1.0);
            let got = ridden_corner(
                LogicalPoint {
                    x: corner.0,
                    y: corner.1,
                },
                &ride,
            );
            assert_eq!((got.x.0, got.y.0), want, "{name}");
        }
    }
}
