//! A window in a minimise flight: drawn shrunk (the Scale effect) or cut into strips (the
//! genie). Which one is `windows.minimise_effect`'s choice; where each strip goes is
//! `casement_motion::genie`'s arithmetic, turned here into physical pixels for smithay's
//! wrappers (`strip.rs`).

use casement_core::geometry::{LogicalRect, LogicalSize, SubPoint, SubPx, SubSize};
use casement_core::settings::MinimiseEffect;
use casement_motion::{Flight, PlacedWindow};

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

#[cfg(test)]
mod tests {
    use super::*;
    use casement_core::geometry::LogicalPoint;
    use ds_core::vocab::Fraction;

    fn rect(x: i32, y: i32, w: u32, h: u32) -> LogicalRect {
        LogicalRect::new(LogicalPoint { x, y }, LogicalSize { w, h })
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
    fn a_window_at_rest_is_placed_where_its_flight_began() {
        let (at, size) = rest_place(&flight(500));
        assert_eq!((at.x, at.y), (SubPx(24_000), SubPx(24_000)));
        assert_eq!((size.w, size.h), (SubPx(96_000), SubPx(72_000)));
    }
}
