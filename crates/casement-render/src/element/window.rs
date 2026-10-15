//! A window's surface tree as elements.

use super::CasementElement;
use super::RoundedClipElement;
use super::blur_v0;
use super::flight::{drawn_content, scale_ratio};
use super::geometry::{geometry_origin, geometry_size};
use super::rounded_clip::ClipShape;
use crate::compositor::Compositor;
use crate::scene::OutputView;
use crate::scene::convert::on_output;
use crate::texture_book::TextureBook;
use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize, SubPoint, SubPx};
use casement_core::settings::MinimiseEffect;
use casement_motion::PlacedWindow;
use ds_core::vocab::Fraction;
use smithay::backend::renderer::element::Kind;
use smithay::backend::renderer::element::surface::{
    WaylandSurfaceRenderElement, render_elements_from_surface_tree,
};
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::utils::Scale;

/// `placed`'s surface tree (subsurfaces, then the root) drawn at its sampled place and opacity
/// on `view`, front first, with its blur v0 behind it when it asked for one. Empty when the
/// book has no surface for it.
///
/// The tree is drawn so that the root's window geometry origin lands at the sampled place, at
/// its committed size, except while the window is in a minimise flight: then every surface of
/// the tree is scaled about that place to the sampled size (smithay's surface-tree helper takes
/// the scale per axis, so damage, opaque regions and the source rectangle follow from it), and
/// the blur behind it is left out. A sampled size that differs from the committed one in any
/// other case (a resize in flight) shows once the client commits it.
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
    let ratio = flight_ratio(placed, &entry.root);
    let surfaces = window_surfaces(placed, ratio, view, book, renderer);
    let mut elements: Vec<CasementElement<R>> = surfaces
        .into_iter()
        .map(|surface| match clip {
            Some(shape) => CasementElement::Clip(RoundedClipElement::new(surface, shape)),
            None => CasementElement::Surface(surface),
        })
        .collect();
    if ratio == (1.0, 1.0) {
        let origin = scaled_origin(&entry.root, placed.at, ratio);
        elements.extend(blur_v0::behind(&entry.root, origin, view).map(CasementElement::Blur));
    }
    elements
}

/// `placed`'s surface tree as surface elements, front first, stretched by `ratio` about the
/// window geometry's place (`(1.0, 1.0)` draws it as committed). Empty when the book has no
/// surface for it.
pub fn window_surfaces<R: Compositor>(
    placed: &PlacedWindow,
    ratio: (f64, f64),
    view: &OutputView,
    book: &TextureBook,
    renderer: &mut R,
) -> Vec<WaylandSurfaceRenderElement<R>> {
    let Some(entry) = book.window(&placed.window) else {
        return Vec::new();
    };
    let origin = scaled_origin(&entry.root, placed.at, ratio);
    let scale = view.scale.as_f64();
    render_elements_from_surface_tree(
        renderer,
        &entry.root,
        on_output(origin, view),
        Scale::from((scale * ratio.0, scale * ratio.1)),
        alpha(placed.opacity),
        Kind::Unspecified,
    )
}

/// How far a window in flight is stretched: its sampled size over its real content size. A
/// window not in flight is drawn as committed.
pub fn flight_ratio(placed: &PlacedWindow, root: &WlSurface) -> (f64, f64) {
    match placed.flight {
        Some(_) => scale_ratio(window_content(placed, root).size, placed.size),
        None => (1.0, 1.0),
    }
}

/// Where `root`'s top-left is when its window geometry's top-left is at `at` and the tree is
/// stretched by `ratio` about that point: the geometry's offset inside the tree stretches too.
fn scaled_origin(root: &WlSurface, at: SubPoint, ratio: (f64, f64)) -> SubPoint {
    let loc = geometry_origin(root);
    let back = |offset: i32, by: f64| (f64::from(SubPx::of_logical(offset).0) * by).round() as i32;
    SubPoint {
        x: SubPx(at.x.0.saturating_sub(back(loc.x, ratio.0))),
        y: SubPx(at.y.0.saturating_sub(back(loc.y, ratio.1))),
    }
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

/// Where `placed`'s content is drawn under `effect`: [`window_content`], at the sampled size
/// while a window in a minimise flight is scaled to it. The frame goes round this, and a title
/// is made for its titlebar.
pub fn window_drawn(
    placed: &PlacedWindow,
    root: &WlSurface,
    effect: MinimiseEffect,
) -> LogicalRect {
    drawn_content(placed, window_content(placed, root), effect)
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
