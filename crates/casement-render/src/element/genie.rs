//! A window in a Genie flight: the window as it stands at rest, cut into the horizontal strips
//! of `casement_motion::genie` and each strip drawn where the curve says (`strip.rs`).
//!
//! Everything the window has rides along. The titlebar and its title are cut like the content.
//! The rounded clip is applied to the strips that reach the bottom corners, with the mask of the
//! window at rest (`clip_strip.rs`). The shadow is cut into the same strips, each reaching
//! sideways by the shadow's margin (and the first and last also above and below), so the halo
//! follows the bent silhouette. The blur is one element over the strips' rectangles. Strips meet
//! on one rounded edge and never overlap, so a translucent window is blended once everywhere.
//! The window's popups are not drawn (FINDINGS, `wave-2026-10-15/flight2`).

use super::clip_strip::ClipStripElement;
use super::flight::at_rest;
use super::frame::SHADOW_MARGIN;
use super::shared::SharedElement;
use super::strip::{Margins, StripElement, StripPlan};
use super::window::{alpha, window_blur, window_content, window_surfaces};
use super::{BlurV0Element, CasementElement, ClipShape, FrameDraw, TitleDraw, frame_elements};
use crate::compositor::Compositor;
use crate::scene::OutputView;
use crate::texture_book::TextureBook;
use casement_core::geometry::LogicalRect;
use casement_core::window::FrameStyle;
use casement_motion::genie::{MAX_STRIPS, RectF, STRIPS, TOLERANCE, strips_fine};
use casement_motion::{Flight, PlacedWindow};
use smithay::backend::renderer::element::memory::MemoryRenderBufferRenderElement;
use smithay::backend::renderer::element::surface::WaylandSurfaceRenderElement;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::utils::{Physical, Point, Rectangle};

/// The strips of the window whose visible rectangle (frame included) is `visual` at `progress`
/// (0 to 1) of the flight to `tile`, in `view`'s physical pixels. Empty when the window has no
/// area. Pure arithmetic.
///
/// The cut is finer where the curve is steep (`strips_fine`), but never finer than one strip
/// per physical row of the window. Two strips share one rounded edge between them, so they
/// meet without a gap and without an overlap.
pub fn strip_plans(
    visual: LogicalRect,
    tile: LogicalRect,
    progress: f64,
    view: &OutputView,
) -> Vec<StripPlan> {
    let scale = view.scale.as_f64();
    let (left, top) = (f64::from(view.area.origin.x), f64::from(view.area.origin.y));
    let physical = |r: RectF| RectF {
        x: (r.x - left) * scale,
        y: (r.y - top) * scale,
        w: r.w * scale,
        h: r.h * scale,
    };
    let src = physical(rect_f(visual));
    if src.w < 1.0 || src.h < 1.0 {
        return Vec::new();
    }
    let rows = (src.h.floor() as usize).max(1);
    let all = strips_fine(
        rect_f(visual),
        rect_f(tile),
        progress,
        STRIPS.min(rows),
        TOLERANCE,
        MAX_STRIPS.min(rows),
    );
    let drawn: Vec<RectF> = all.iter().map(|strip| physical(strip.dst)).collect();
    // One rounded edge between two strips, used by both.
    let edges: Vec<i32> = drawn
        .iter()
        .map(|d| d.y.round() as i32)
        .chain(drawn.last().map(|d| (d.y + d.h).round() as i32))
        .collect();
    let origin: Point<i32, Physical> = (src.x.round() as i32, src.y.round() as i32).into();
    all.iter()
        .enumerate()
        .filter_map(|(index, strip)| {
            let d = drawn[index];
            let (x0, x1) = (d.x.round() as i32, (d.x + d.w).round() as i32);
            let (y0, y1) = (edges[index], edges[index + 1]);
            let source_h = src.h * (strip.bottom - strip.top);
            if x1 <= x0 || y1 <= y0 || source_h <= 1e-3 {
                return None;
            }
            let ratio = (d.w / src.w, d.h / source_h);
            // The shift is one rounded difference, so a strip at rest (where the difference is
            // float noise round zero) is not moved by a half-pixel tie between two roundings.
            let shift_y = (d.y - (src.y + strip.top * src.h * ratio.1)).round() as i32;
            Some(StripPlan {
                index,
                origin,
                scale: ratio,
                band: Rectangle::new((origin.x, y0 - shift_y).into(), (x1 - x0, y1 - y0).into()),
                shift: (x0 - origin.x, shift_y).into(),
                source: (src.y + strip.top * src.h, src.y + strip.bottom * src.h),
            })
        })
        .collect()
}

/// What a genie cuts: the pieces of one window as it stands at rest.
#[derive(Debug)]
pub struct GenieParts<R: Compositor> {
    /// The flight (its tile and how far along it is).
    pub flight: Flight,
    /// The window's visible rectangle, frame included, global logical pixels.
    pub visual: LogicalRect,
    /// The titlebar and title (in front of the surfaces).
    pub front: Vec<MemoryRenderBufferRenderElement<R>>,
    /// The window's surface tree, front first.
    pub surfaces: Vec<WaylandSurfaceRenderElement<R>>,
    /// The shadow's pieces (behind everything).
    pub shadow: Vec<MemoryRenderBufferRenderElement<R>>,
    /// The blur behind the surfaces, as it is at rest.
    pub blur: Option<BlurV0Element>,
    /// The rounded clip of the window at rest, when it has one.
    pub clip: Option<ClipShape>,
}

/// The genie's elements, front first: each strip's titlebar and surfaces, the blur, then the
/// shadow's strips.
pub fn genie_elements<R: Compositor>(
    parts: GenieParts<R>,
    view: &OutputView,
) -> Vec<CasementElement<R>> {
    let progress = f64::from(parts.flight.progress.0.min(1000)) / 1000.0;
    let scale = view.scale.as_f64();
    let plans = strip_plans(parts.visual, parts.flight.tile, progress, view);
    let front = shared(parts.front);
    let shadow = shared(parts.shadow);
    let surfaces = shared(parts.surfaces);
    let mut elements = Vec::new();
    for plan in &plans {
        elements.extend(
            front
                .iter()
                .filter_map(|piece| StripElement::new(piece.clone(), scale, plan))
                .map(CasementElement::MemoryStrip),
        );
        let clipped = parts.clip.filter(|shape| shape.touches(plan.source));
        for piece in &surfaces {
            match clipped {
                Some(shape) => elements.extend(
                    ClipStripElement::new(piece.clone(), scale, plan, shape)
                        .map(CasementElement::ClipStrip),
                ),
                None => elements.extend(
                    StripElement::new(piece.clone(), scale, plan)
                        .map(CasementElement::SurfaceStrip),
                ),
            }
        }
    }
    if let Some(blur) = parts.blur {
        let cut = blur.remapped(|rect| {
            plans
                .iter()
                .filter_map(|plan| plan.mapped(rect))
                .collect::<Vec<_>>()
        });
        if !cut.region().is_empty() {
            elements.push(CasementElement::Blur(cut));
        }
    }
    let reach = (f64::from(SHADOW_MARGIN) * scale).ceil() as i32;
    let last = plans.len().saturating_sub(1);
    for (position, plan) in plans.iter().enumerate() {
        let reaching = plan.widened(halo(position, last, reach));
        elements.extend(
            shadow
                .iter()
                .filter_map(|piece| StripElement::new(piece.clone(), scale, &reaching))
                .map(CasementElement::MemoryStrip),
        );
    }
    elements
}

/// Each of `list` as an element many strips can hold.
fn shared<E>(list: Vec<E>) -> Vec<SharedElement<E>> {
    list.into_iter().map(SharedElement::new).collect()
}

/// How far the shadow's strip at `position` (of 0 to `last`) reaches past its band: `reach`
/// sideways, and the first strip also above, the last also below.
fn halo(position: usize, last: usize, reach: i32) -> Margins {
    let when = |hit: bool| match hit {
        true => reach,
        false => 0,
    };
    Margins {
        side: reach,
        top: when(position == 0),
        bottom: when(position == last),
    }
}

/// What a window in a Genie flight is drawn from.
#[derive(Debug)]
pub struct GenieWindow<'a> {
    /// The window as the frame places it (the genie cuts it as it stands at rest).
    pub placed: &'a PlacedWindow,
    /// The flight.
    pub flight: Flight,
    /// The window's frame, when casement decorates it.
    pub style: Option<&'a FrameStyle>,
    /// Its title raster.
    pub title: Option<&'a TitleDraw>,
    /// The corner radius to clip the surfaces to, in logical pixels, when they are clipped.
    pub clip_radius: Option<u32>,
}

/// A window in a Genie flight as elements: its frame, surfaces, blur, clip and shadow, cut and
/// placed. `root` is the window's surface tree.
pub fn genie_window<R: Compositor>(
    window: GenieWindow<'_>,
    root: &WlSurface,
    view: &OutputView,
    book: &TextureBook,
    renderer: &mut R,
) -> Vec<CasementElement<R>> {
    let GenieWindow {
        placed,
        flight,
        style,
        title,
        clip_radius,
    } = window;
    let rest = at_rest(placed, &flight);
    let content = window_content(&rest, root);
    let (front, back) = style
        .map(|style| {
            let draw = FrameDraw {
                style,
                title,
                content,
                full: content.size,
                opacity: placed.opacity,
            };
            frame_elements(placed.window, draw, view, book, renderer)
        })
        .unwrap_or_default();
    let visual = style.map_or(content, |style| style.around(content).outline());
    let blur = window_blur(&rest, root, ((1.0, 1.0), alpha(placed.opacity)), view);
    let parts = GenieParts {
        flight,
        visual,
        front: memory(front),
        surfaces: window_surfaces(&rest, (1.0, 1.0), view, book, renderer),
        shadow: memory(back),
        blur,
        clip: clip_radius.map(|radius| ClipShape::of_content(content, radius, view)),
    };
    genie_elements(parts, view)
}

/// The raster pieces among `elements`.
fn memory<R: Compositor>(
    elements: Vec<CasementElement<R>>,
) -> Vec<MemoryRenderBufferRenderElement<R>> {
    elements
        .into_iter()
        .filter_map(|element| match element {
            CasementElement::Memory(piece) => Some(piece),
            _ => None,
        })
        .collect()
}

/// `rect` as floats.
fn rect_f(rect: LogicalRect) -> RectF {
    RectF {
        x: f64::from(rect.origin.x),
        y: f64::from(rect.origin.y),
        w: f64::from(rect.size.w),
        h: f64::from(rect.size.h),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use casement_core::geometry::{LogicalPoint, LogicalSize, PhysicalSize};
    use ds_core::geometry::scale::Scale;

    fn rect(x: i32, y: i32, w: u32, h: u32) -> LogicalRect {
        LogicalRect::new(LogicalPoint { x, y }, LogicalSize { w, h })
    }

    fn view(scale: u32) -> OutputView {
        OutputView {
            area: rect(0, 0, 1920, 1080),
            scale: Scale(scale),
            mode: PhysicalSize { w: 1920, h: 1080 },
        }
    }

    const TILE: (i32, i32, u32, u32) = (900, 1040, 48, 30);

    fn tile() -> LogicalRect {
        rect(TILE.0, TILE.1, TILE.2, TILE.3)
    }

    #[test]
    fn a_genie_at_rest_is_one_unscaled_band_per_strip_and_meets_on_shared_edges() {
        let visual = rect(200, 176, 800, 624);
        for scale in [120, 180, 240] {
            let plans = strip_plans(visual, tile(), 0.0, &view(scale));
            assert_eq!(plans.len(), STRIPS, "scale {scale}");
            for plan in &plans {
                assert!((plan.scale.0 - 1.0).abs() < 1e-9, "{plan:?}");
                assert!((plan.scale.1 - 1.0).abs() < 1e-9, "{plan:?}");
                assert_eq!(plan.shift.x, 0);
                assert_eq!(plan.shift.y, 0, "{plan:?}");
            }
            for pair in plans.windows(2) {
                let end = pair[0].band.loc.y + pair[0].band.size.h;
                // no overlap and no gap: a translucent window is blended once
                assert_eq!(end, pair[1].band.loc.y, "scale {scale}");
            }
        }
    }

    #[test]
    fn strips_meet_exactly_at_any_progress_and_scale() {
        let visual = rect(200, 176, 800, 624);
        for scale in [120, 180, 240] {
            for step in 0..=20 {
                let plans = strip_plans(visual, tile(), f64::from(step) / 20.0, &view(scale));
                let rows: Vec<(i32, i32)> = plans
                    .iter()
                    .map(|plan| {
                        let top = plan.band.loc.y + plan.shift.y;
                        (top, top + plan.band.size.h)
                    })
                    .collect();
                for pair in rows.windows(2) {
                    assert_eq!(pair[0].1, pair[1].0, "scale {scale} step {step}: {pair:?}");
                }
            }
        }
    }

    #[test]
    fn the_curve_is_cut_finer_halfway_than_at_rest() {
        let visual = rect(200, 176, 800, 624);
        let at = |p: f64| strip_plans(visual, tile(), p, &view(120)).len();
        assert_eq!(at(0.0), STRIPS);
        assert!(at(0.5) > STRIPS && at(0.5) <= MAX_STRIPS, "{}", at(0.5));
    }

    #[test]
    fn a_short_window_has_no_strip_thinner_than_a_row() {
        let plans = strip_plans(rect(200, 200, 300, 20), tile(), 0.5, &view(120));
        assert!(!plans.is_empty() && plans.len() <= 20, "{}", plans.len());
        assert!(plans.iter().all(|plan| plan.band.size.h >= 1));
    }

    #[test]
    fn the_sources_of_the_strips_cover_the_window_once() {
        let visual = rect(200, 176, 800, 624);
        let plans = strip_plans(visual, tile(), 0.4, &view(180));
        let first = plans[0].source.0;
        let last = plans[plans.len() - 1].source.1;
        assert!((first - 176.0 * 1.5).abs() < 1e-6, "{first}");
        assert!((last - 800.0 * 1.5).abs() < 1e-6, "{last}");
        for pair in plans.windows(2) {
            assert!((pair[0].source.1 - pair[1].source.0).abs() < 1e-6);
        }
    }

    #[test]
    fn a_window_with_no_area_has_no_strips() {
        assert!(strip_plans(rect(0, 0, 0, 0), tile(), 0.5, &view(120)).is_empty());
    }

    #[test]
    fn only_the_ends_of_the_shadow_reach_past_the_window() {
        // name, position, last, margins
        let cases = [
            ("first", 0, 5, (8, 8, 0)),
            ("middle", 2, 5, (8, 0, 0)),
            ("last", 5, 5, (8, 0, 8)),
            ("only", 0, 0, (8, 8, 8)),
        ];
        for (name, position, last, (side, top, bottom)) in cases {
            let got = halo(position, last, 8);
            assert_eq!(
                (got.side, got.top, got.bottom),
                (side, top, bottom),
                "{name}"
            );
        }
    }

    #[test]
    fn the_strips_of_a_solid_window_never_overlap() {
        use crate::element::{Rgba8, solid_rect};
        use casement_core::geometry::PhysicalRect;
        use smithay::backend::renderer::element::{Element, Id};
        use smithay::backend::renderer::utils::CommitCounter;

        let red = Rgba8 {
            r: 255,
            g: 0,
            b: 0,
            a: 255,
        };
        let area = PhysicalRect {
            x: 200,
            y: 176,
            w: 800,
            h: 624,
        };
        for progress in [0.0, 0.3, 0.7, 1.0] {
            let plans = strip_plans(rect(200, 176, 800, 624), tile(), progress, &view(120));
            let drawn: Vec<_> = plans
                .iter()
                .filter_map(|plan| {
                    let solid = solid_rect(Id::new(), area, red, CommitCounter::default());
                    StripElement::new(solid, 1.0, plan)
                })
                .map(|strip| strip.geometry(smithay::utils::Scale::from(1.0)))
                .collect();
            assert!(drawn.len() > 1, "progress {progress}");
            for (index, one) in drawn.iter().enumerate() {
                for other in &drawn[index + 1..] {
                    assert!(
                        one.intersection(*other).is_none(),
                        "progress {progress}: {one:?} and {other:?} overlap"
                    );
                }
            }
        }
    }
}
