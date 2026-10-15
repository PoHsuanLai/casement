//! A window in a minimise flight: drawn shrunk (the Scale effect) or cut into strips (the
//! genie). Which one is `windows.minimise_effect`'s choice; where each strip goes is
//! `casement_motion::genie`'s arithmetic, turned here into physical pixels for smithay's
//! wrappers (`strip.rs`).

use super::CasementElement;
use super::shared::SharedElement;
use super::strip::{StripElement, StripPlan};
use crate::compositor::Compositor;
use crate::scene::OutputView;
use casement_core::geometry::{LogicalRect, LogicalSize, SubPoint, SubPx, SubSize};
use casement_core::settings::MinimiseEffect;
use casement_motion::genie::{RectF, STRIPS, strips};
use casement_motion::{Flight, PlacedWindow};
use smithay::backend::renderer::element::memory::MemoryRenderBufferRenderElement;
use smithay::backend::renderer::element::surface::WaylandSurfaceRenderElement;
use smithay::utils::{Physical, Point, Rectangle};

/// How a window is drawn this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FlightDraw {
    /// As its client committed it, at its sampled place.
    Still,
    /// Shrunk (or grown) to its sampled size about its place.
    Scaled,
    /// Cut into strips that follow the genie's curve.
    Genie(Flight),
}

/// How a window with `flight` is drawn under `effect`: a window not in flight is
/// [`FlightDraw::Still`].
pub fn choose(effect: MinimiseEffect, flight: Option<Flight>) -> FlightDraw {
    match (flight, effect) {
        (None, _) => FlightDraw::Still,
        (Some(_), MinimiseEffect::Scale) => FlightDraw::Scaled,
        (Some(flight), MinimiseEffect::Genie) => FlightDraw::Genie(flight),
    }
}

/// How much a window of `content` size is stretched to fill `size`, per axis (1 when it has no
/// size to stretch from).
///
/// ```
/// use casement_core::geometry::{LogicalSize, SubPx, SubSize};
/// use casement_render::scale_ratio;
///
/// let content = LogicalSize { w: 800, h: 600 };
/// let half = SubSize { w: SubPx(48_000), h: SubPx(36_000) };
/// assert_eq!(scale_ratio(content, half), (0.5, 0.5));
/// ```
pub fn scale_ratio(content: LogicalSize, size: SubSize) -> (f64, f64) {
    let axis = |drawn: SubPx, committed: u32| match committed {
        0 => 1.0,
        committed => f64::from(drawn.0.max(0)) / (f64::from(committed) * 120.0),
    };
    (axis(size.w, content.w), axis(size.h, content.h))
}

/// Where `placed`'s content is drawn under `effect`, given where it is at its real size
/// (`content`): the same place at the sampled size when it is [`FlightDraw::Scaled`], else
/// `content`. The frame is drawn round this, and the title is made for its titlebar.
pub fn drawn_content(
    placed: &PlacedWindow,
    content: LogicalRect,
    effect: MinimiseEffect,
) -> LogicalRect {
    scaled_content(choose(effect, placed.flight), content, placed.size)
}

/// `content` for a window drawn as `draw`: at the sampled `size` when it is scaled.
pub fn scaled_content(draw: FlightDraw, content: LogicalRect, size: SubSize) -> LogicalRect {
    let side = |px: SubPx| u32::try_from(SubPx(px.0.max(0)).round_logical()).unwrap_or(0);
    match draw {
        FlightDraw::Scaled => LogicalRect::new(
            content.origin,
            LogicalSize {
                w: side(size.w),
                h: side(size.h),
            },
        ),
        FlightDraw::Still | FlightDraw::Genie(_) => content,
    }
}

/// `placed` standing where its flight began (or ends): the genie cuts the window as it is at
/// rest, so its surfaces and frame are placed there, at their own size.
pub fn at_rest(placed: &PlacedWindow, flight: &Flight) -> PlacedWindow {
    let (at, size) = rest_place(flight);
    PlacedWindow {
        at,
        size,
        flight: None,
        ..*placed
    }
}

/// Where the window of `flight` stands at rest, and its size, in sub-pixels.
pub fn rest_place(flight: &Flight) -> (SubPoint, SubSize) {
    let side = |n: u32| SubPx::of_logical(i32::try_from(n).unwrap_or(i32::MAX));
    let at = SubPoint {
        x: SubPx::of_logical(flight.window.origin.x),
        y: SubPx::of_logical(flight.window.origin.y),
    };
    let size = SubSize {
        w: side(flight.window.size.w),
        h: side(flight.window.size.h),
    };
    (at, size)
}

/// The strips of the window whose visible rectangle (frame included) is `visual` at `progress`
/// (0 to 1) of the flight to `tile`, in `view`'s physical pixels. Empty when the window has no
/// area. Pure arithmetic.
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
    let all = strips(rect_f(visual), rect_f(tile), progress, STRIPS);
    let last = all.len().saturating_sub(1);
    let origin: Point<i32, Physical> = (src.x.round() as i32, src.y.round() as i32).into();
    all.iter()
        .enumerate()
        .filter_map(|(index, strip)| {
            let d = physical(strip.dst);
            let (x0, x1) = (d.x.round() as i32, (d.x + d.w).round() as i32);
            let (y0, y1) = (d.y.round() as i32, (d.y + d.h).round() as i32);
            let source_h = src.h * (strip.bottom - strip.top);
            if x1 <= x0 || y1 <= y0 || source_h < 1.0 {
                return None;
            }
            let ratio = (d.w / src.w, d.h / source_h);
            // The shift is one rounded difference, so a strip at rest (where the difference is
            // float noise round zero) is not moved by a half-pixel tie between two roundings.
            let shift_y = (d.y - (src.y + strip.top * src.h * ratio.1)).round() as i32;
            let band_top = y0 - shift_y;
            // Every strip but the last runs one pixel into the next, so rounding leaves no seam.
            let overlap = match index == last {
                true => 0,
                false => 1,
            };
            Some(StripPlan {
                index,
                origin,
                scale: ratio,
                band: Rectangle::new(
                    (origin.x, band_top).into(),
                    (x1 - x0, y1 - y0 + overlap).into(),
                ),
                shift: (x0 - origin.x, shift_y).into(),
            })
        })
        .collect()
}

/// The genie's elements: `front` (the frame's titlebar and title) and `surfaces` (the window's
/// tree), both as they are at rest, each cut into the strips of [`strip_plans`]. Front first.
pub fn genie_elements<R: Compositor>(
    flight: &Flight,
    visual: LogicalRect,
    front: Vec<MemoryRenderBufferRenderElement<R>>,
    surfaces: Vec<WaylandSurfaceRenderElement<R>>,
    view: &OutputView,
) -> Vec<CasementElement<R>> {
    let progress = f64::from(flight.progress.0.min(1000)) / 1000.0;
    let scale = view.scale.as_f64();
    let front: Vec<_> = front.into_iter().map(SharedElement::new).collect();
    let surfaces: Vec<_> = surfaces.into_iter().map(SharedElement::new).collect();
    let mut elements = Vec::new();
    for plan in strip_plans(visual, flight.tile, progress, view) {
        elements.extend(
            front
                .iter()
                .filter_map(|piece| StripElement::new(piece.clone(), scale, &plan))
                .map(CasementElement::MemoryStrip),
        );
        elements.extend(
            surfaces
                .iter()
                .filter_map(|piece| StripElement::new(piece.clone(), scale, &plan))
                .map(CasementElement::SurfaceStrip),
        );
    }
    elements
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
    use casement_core::geometry::{LogicalPoint, PhysicalSize};
    use ds_core::geometry::scale::Scale;
    use ds_core::vocab::Fraction;

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

    fn flight(progress: u16) -> Flight {
        Flight::new(
            rect(200, 200, 800, 600),
            rect(900, 1040, 48, 30),
            Fraction(progress),
        )
    }

    #[test]
    fn the_effect_decides_how_a_flight_is_drawn() {
        let flying = Some(flight(400));
        // name, effect, flight, drawn
        let cases = [
            (
                "genie",
                MinimiseEffect::Genie,
                flying,
                FlightDraw::Genie(flight(400)),
            ),
            ("scale", MinimiseEffect::Scale, flying, FlightDraw::Scaled),
            (
                "genie, not flying",
                MinimiseEffect::Genie,
                None,
                FlightDraw::Still,
            ),
            (
                "scale, not flying",
                MinimiseEffect::Scale,
                None,
                FlightDraw::Still,
            ),
        ];
        for (name, effect, flight, want) in cases {
            assert_eq!(choose(effect, flight), want, "{name}");
        }
        assert_eq!(MinimiseEffect::default(), MinimiseEffect::Genie);
    }

    #[test]
    fn a_window_is_stretched_by_its_sampled_size_over_its_content() {
        // name, content, sampled size (sub-pixels), ratio
        let cases = [
            ("same", (800, 600), (96_000, 72_000), (1.0, 1.0)),
            ("half", (800, 600), (48_000, 36_000), (0.5, 0.5)),
            ("tile", (800, 600), (5_760, 3_600), (0.06, 0.05)),
            ("one axis", (800, 600), (96_000, 36_000), (1.0, 0.5)),
            ("empty content", (0, 0), (96_000, 72_000), (1.0, 1.0)),
            ("negative size", (800, 600), (-5, -5), (0.0, 0.0)),
        ];
        for (name, content, size, want) in cases {
            let got = scale_ratio(
                LogicalSize {
                    w: content.0,
                    h: content.1,
                },
                SubSize {
                    w: SubPx(size.0),
                    h: SubPx(size.1),
                },
            );
            assert!(
                (got.0 - want.0).abs() < 1e-9 && (got.1 - want.1).abs() < 1e-9,
                "{name}: {got:?}"
            );
        }
    }

    #[test]
    fn only_a_scaled_window_is_drawn_at_its_sampled_size() {
        let content = rect(200, 200, 800, 600);
        let half = SubSize {
            w: SubPx(48_000),
            h: SubPx(36_000),
        };
        let small = rect(200, 200, 400, 300);
        assert_eq!(scaled_content(FlightDraw::Scaled, content, half), small);
        assert_eq!(scaled_content(FlightDraw::Still, content, half), content);
        let genie = FlightDraw::Genie(flight(500));
        assert_eq!(scaled_content(genie, content, half), content);
    }

    #[test]
    fn a_genie_at_rest_is_one_unscaled_band_per_strip_and_shares_every_edge() {
        let visual = rect(200, 176, 800, 624);
        let tile = rect(900, 1040, 48, 30);
        for scale in [120, 180, 240] {
            let plans = strip_plans(visual, tile, 0.0, &view(scale));
            assert_eq!(plans.len(), STRIPS, "scale {scale}");
            for plan in &plans {
                assert!((plan.scale.0 - 1.0).abs() < 1e-9, "{plan:?}");
                assert!((plan.scale.1 - 1.0).abs() < 1e-9, "{plan:?}");
                assert_eq!(plan.shift.x, 0);
                assert_eq!(plan.shift.y, 0, "{plan:?}");
            }
            for pair in plans.windows(2) {
                let end = pair[0].band.loc.y + pair[0].band.size.h;
                // each strip but the last runs one pixel into the next
                assert_eq!(end - 1, pair[1].band.loc.y, "scale {scale}");
            }
        }
    }

    #[test]
    fn strips_land_without_a_gap_at_any_progress() {
        let visual = rect(200, 176, 800, 624);
        let tile = rect(900, 1040, 48, 30);
        for step in 0..=20 {
            let plans = strip_plans(visual, tile, f64::from(step) / 20.0, &view(120));
            let tops: Vec<(i32, i32)> = plans
                .iter()
                .map(|plan| {
                    let top = plan.band.loc.y + plan.shift.y;
                    (top, top + plan.band.size.h)
                })
                .collect();
            for pair in tops.windows(2) {
                assert!(
                    pair[1].0 <= pair[0].1,
                    "a gap at step {step}: {pair:?} (a strip starts at or before the last one ends)"
                );
            }
        }
    }

    #[test]
    fn a_window_with_no_area_has_no_strips() {
        let tile = rect(900, 1040, 48, 30);
        assert!(strip_plans(rect(0, 0, 0, 0), tile, 0.5, &view(120)).is_empty());
    }

    #[test]
    fn a_window_at_rest_is_placed_where_its_flight_began() {
        let (at, size) = rest_place(&flight(500));
        assert_eq!((at.x, at.y), (SubPx(24_000), SubPx(24_000)));
        assert_eq!((size.w, size.h), (SubPx(96_000), SubPx(72_000)));
    }
}
