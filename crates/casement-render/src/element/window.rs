//! A window's surface tree as elements.

use super::CasementElement;
use super::RoundedClipElement;
use super::blur_v0;
use super::geometry::{geometry_size, surface_origin};
use super::rounded_clip::ClipShape;
use crate::compositor::Compositor;
use crate::scene::OutputView;
use crate::scene::convert::on_output;
use crate::texture_book::TextureBook;
use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize};
use casement_motion::PlacedWindow;
use ds_core::vocab::Fraction;
use smithay::backend::renderer::element::Kind;
use smithay::backend::renderer::element::surface::{
    WaylandSurfaceRenderElement, render_elements_from_surface_tree,
};
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;

/// `placed`'s surface tree (subsurfaces, then the root) drawn at its sampled place and opacity
/// on `view`, front first, with its blur v0 behind it when it asked for one. Empty when the
/// book has no surface for it.
///
/// The tree is drawn so that the root's window geometry origin lands at the sampled place
/// ([`surface_origin`]), at its committed size: the closed element set has no rescaling wrapper,
/// so a sampled size that differs from the committed one (a resize in flight) shows once the
/// client commits it (FINDINGS: P3-Z adds the wrapper with the effect elements).
///
/// With a `clip` every surface of the tree (the root and its subsurfaces) is drawn inside that
/// shape, a window's rounded bottom corners; the window's popups are not part of this tree.
pub fn window_elements<R: Compositor>(
    placed: &PlacedWindow,
    clip: Option<ClipShape>,
    view: &OutputView,
    book: &TextureBook,
    renderer: &mut R,
) -> Vec<CasementElement<R>> {
    let Some(entry) = book.window(&placed.window) else {
        return Vec::new();
    };
    let origin = surface_origin(&entry.root, placed.at);
    let surfaces: Vec<WaylandSurfaceRenderElement<R>> = render_elements_from_surface_tree(
        renderer,
        &entry.root,
        on_output(origin, view),
        view.scale.as_f64(),
        alpha(placed.opacity),
        Kind::Unspecified,
    );
    let mut elements: Vec<CasementElement<R>> = surfaces
        .into_iter()
        .map(|surface| match clip {
            Some(shape) => CasementElement::Clip(RoundedClipElement::new(surface, shape)),
            None => CasementElement::Surface(surface),
        })
        .collect();
    elements.extend(blur_v0::behind(&entry.root, origin, view).map(CasementElement::Blur));
    elements
}

/// Where `placed`'s content is, in global logical pixels: its place, at the size of its window
/// geometry (its committed size when it declared none). The frame is drawn round it and the
/// hit test reads it, so the two agree.
pub fn window_content(placed: &PlacedWindow, root: &WlSurface) -> LogicalRect {
    let size = geometry_size(root).map_or(placed.committed, |size| LogicalSize {
        w: u32::try_from(size.w).unwrap_or(0),
        h: u32::try_from(size.h).unwrap_or(0),
    });
    LogicalRect::new(
        LogicalPoint {
            x: placed.at.x.round_logical(),
            y: placed.at.y.round_logical(),
        },
        size,
    )
}

/// An opacity as smithay's alpha: thousandths, at most one.
pub(crate) fn alpha(opacity: Fraction) -> f32 {
    f32::from(opacity.0.min(1000)) / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opacity_becomes_alpha_held_to_one() {
        assert_eq!(alpha(Fraction(0)), 0.0);
        assert_eq!(alpha(Fraction(350)), 0.35);
        assert_eq!(alpha(Fraction(1000)), 1.0);
        assert_eq!(alpha(Fraction(1400)), 1.0);
    }
}
