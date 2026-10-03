//! The popups of a window or layer surface as elements.

use super::CasementElement;
use crate::compositor::Compositor;
use crate::scene::OutputView;
use casement_core::geometry::LogicalPoint;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;

/// Every popup whose root is `root` (smithay's popup tree), placed relative to the root's
/// top-left at `origin` (global logical), front first.
pub fn popup_elements<R: Compositor>(
    root: &WlSurface,
    origin: LogicalPoint,
    view: &OutputView,
    renderer: &mut R,
) -> Vec<CasementElement<R>> {
    let _ = (root, origin, view, renderer);
    todo!("P0-R-scene: popup_elements")
}
