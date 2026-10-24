//! The genie's strip geometry: a pure function of the window's rectangle, the dock tile's
//! rectangle and how far along the flight is. The renderer cuts the window into horizontal
//! strips and draws each one where this says.
//!
//! A row of the window (`v` from 0 at the top to 1 at the bottom) has gone a fraction of the way
//! to the tile, and the lower rows lead: the bottom narrows toward the tile's width first, along
//! an S curve, while the top is still in place, and then the rest of the window slides down into
//! the tile. The rows' places are one function of `v`, so the strips share their edges: no gap
//! and no overlap by construction.
//!
//! ```
//! use casement_motion::genie::{RectF, STRIPS, strips};
//!
//! let window = RectF { x: 100.0, y: 100.0, w: 400.0, h: 300.0 };
//! let tile = RectF { x: 900.0, y: 1040.0, w: 48.0, h: 30.0 };
//! let at_rest = strips(window, tile, 0.0, STRIPS);
//! assert_eq!(at_rest.len(), STRIPS);
//! assert_eq!(at_rest[0].dst.y, 100.0);
//! ```

/// How many strips a window is cut into where the curve is gentle.
pub const STRIPS: usize = 48;

/// The most strips a window is cut into where the curve is steep (see [`strips_fine`]).
pub const MAX_STRIPS: usize = 120;

/// The most pieces one base strip is cut into, so no piece is thinner than a base strip over this.
const MAX_SPLIT: usize = 32;

/// How far the stepped silhouette may stray from the curve, in logical pixels, when the budget
/// of [`MAX_STRIPS`] allows.
pub const TOLERANCE: f64 = 1.0;

/// How much later the top of the window leaves than the bottom, as a share of the whole flight.
const LAG: f64 = 0.6;

/// A rectangle in floating logical pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RectF {
    /// The left edge.
    pub x: f64,
    /// The top edge.
    pub y: f64,
    /// The width.
    pub w: f64,
    /// The height.
    pub h: f64,
}

impl RectF {
    /// The horizontal middle.
    fn centre(self) -> f64 {
        self.x + self.w / 2.0
    }
}

/// One strip: the band of the window it shows and where that band is drawn.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct Strip {
    /// The band's top, as a share of the window's height (0 at the top).
    pub top: f64,
    /// The band's bottom, as a share of the window's height.
    pub bottom: f64,
    /// Where the band is drawn.
    pub dst: RectF,
}

/// Where one row of the window is: its top edge, its middle and its width.
#[derive(Debug, Clone, Copy)]
struct Row {
    y: f64,
    centre: f64,
    width: f64,
}

/// `count` strips of `window` at `progress` (0 is the window as it is, 1 is all of it inside
/// `tile`; held to that range), top strip first. At least one strip.
pub fn strips(window: RectF, tile: RectF, progress: f64, count: usize) -> Vec<Strip> {
    let count = count.max(1);
    let shares: Vec<f64> = (0..=count)
        .map(|index| index as f64 / count as f64)
        .collect();
    strips_at(window, tile, progress, &shares)
}

/// The strips of `strips(.., base)`, with the steep ones cut finer so the stepped silhouette
/// stays within `tolerance` logical pixels of the curve where the budget allows.
///
/// A strip is a rectangle, and the curve through it is slanted: the edge of a strip is off the
/// curve by up to half the distance the curve's edge moves across the strip. So a base strip
/// whose rows' edges are `jump` apart is cut in `ceil(jump / (2 * tolerance))` pieces, at
/// equal heights. When the pieces together would pass `limit` strips, the extra cuts are shared
/// out in proportion to what each strip wanted. At rest nothing is cut: exactly `base` strips.
///
/// ```
/// use casement_motion::genie::{RectF, STRIPS, strips_fine};
///
/// let window = RectF { x: 100.0, y: 100.0, w: 400.0, h: 300.0 };
/// let tile = RectF { x: 900.0, y: 1040.0, w: 48.0, h: 30.0 };
/// assert_eq!(strips_fine(window, tile, 0.0, STRIPS, 1.0, 120).len(), STRIPS);
/// assert!(strips_fine(window, tile, 0.5, STRIPS, 1.0, 120).len() > STRIPS);
/// ```
pub fn strips_fine(
    window: RectF,
    tile: RectF,
    progress: f64,
    base: usize,
    tolerance: f64,
    limit: usize,
) -> Vec<Strip> {
    let base = base.max(1);
    let progress = progress.clamp(0.0, 1.0);
    let step = tolerance.max(0.25) * 2.0;
    let edges: Vec<f64> = (0..=base).map(|index| index as f64 / base as f64).collect();
    let sides = |v: f64| {
        let row = row_at(window, tile, progress, v);
        (row.centre - row.width / 2.0, row.centre + row.width / 2.0)
    };
    let wanted: Vec<usize> = edges
        .windows(2)
        .map(|pair| {
            let (above, below) = (sides(pair[0]), sides(pair[1]));
            let jump = (below.0 - above.0).abs().max((below.1 - above.1).abs());
            ((jump / step).ceil() as usize).clamp(1, MAX_SPLIT)
        })
        .collect();
    let extra: usize = wanted.iter().map(|n| n - 1).sum();
    let budget = limit.saturating_sub(base);
    let pieces: Vec<usize> = wanted
        .iter()
        .map(|n| match extra > budget {
            true => 1 + (n - 1) * budget / extra,
            false => *n,
        })
        .collect();
    let shares: Vec<f64> = edges
        .windows(2)
        .zip(&pieces)
        .flat_map(|(pair, count)| {
            (0..*count).map(move |at| pair[0] + (pair[1] - pair[0]) * at as f64 / *count as f64)
        })
        .chain([1.0])
        .collect();
    strips_at(window, tile, progress, &shares)
}

/// `strips` with each one that covers no whole row (`row` rounds a logical y to its physical row)
/// joined to the next, and a last such strip to the one before it: every strip keeps its rows.
pub fn whole_rows(strips: Vec<Strip>, row: impl Fn(f64) -> i32) -> Vec<Strip> {
    let rows_of = |strip: &Strip| row(strip.dst.y + strip.dst.h) - row(strip.dst.y);
    let mut kept: Vec<Strip> = Vec::with_capacity(strips.len());
    for strip in strips {
        match kept.last_mut() {
            Some(last) if rows_of(last) <= 0 => *last = joined(*last, strip),
            _ => kept.push(strip),
        }
    }
    if kept.len() > 1 && kept.last().is_some_and(|last| rows_of(last) <= 0) {
        let last = kept.pop().expect("a last strip");
        let before = kept.pop().expect("a strip before it");
        kept.push(joined(before, last));
    }
    kept
}

/// One strip from `upper` and the `lower` strip just under it: the rows of both, drawn over the
/// width of both.
fn joined(upper: Strip, lower: Strip) -> Strip {
    let left = upper.dst.x.min(lower.dst.x);
    let right = (upper.dst.x + upper.dst.w).max(lower.dst.x + lower.dst.w);
    Strip {
        top: upper.top,
        bottom: lower.bottom,
        dst: RectF {
            x: left,
            y: upper.dst.y,
            w: right - left,
            h: lower.dst.y + lower.dst.h - upper.dst.y,
        },
    }
}

/// The strips between consecutive `shares` (rows as shares of the height, rising from 0 to 1).
fn strips_at(window: RectF, tile: RectF, progress: f64, shares: &[f64]) -> Vec<Strip> {
    let progress = progress.clamp(0.0, 1.0);
    let mut floor = f64::NEG_INFINITY;
    let rows: Vec<Row> = shares
        .iter()
        .map(|share| {
            let row = row_at(window, tile, progress, *share);
            // A tile above the window's rows would fold the window over itself; rows never go up.
            floor = floor.max(row.y);
            Row { y: floor, ..row }
        })
        .collect();
    rows.windows(2)
        .zip(shares.windows(2))
        .map(|(pair, share)| {
            let (above, below) = (pair[0], pair[1]);
            let width = (above.width + below.width) / 2.0;
            let centre = (above.centre + below.centre) / 2.0;
            Strip {
                top: share[0],
                bottom: share[1],
                dst: RectF {
                    x: centre - width / 2.0,
                    y: above.y,
                    w: width,
                    h: below.y - above.y,
                },
            }
        })
        .collect()
}

/// Row `v` of the window at `progress`.
fn row_at(window: RectF, tile: RectF, progress: f64, v: f64) -> Row {
    let went = reach(v, progress);
    let lerp = |from: f64, to: f64| from + (to - from) * went;
    Row {
        y: lerp(window.y + v * window.h, tile.y + v * tile.h),
        centre: lerp(window.centre(), tile.centre()),
        width: lerp(window.w, tile.w),
    }
}

/// How far toward the tile row `v` has gone at `progress`: the bottom row first, the top row
/// last, all of them there at 1, each on a smooth step.
fn reach(v: f64, progress: f64) -> f64 {
    let t = (progress * (1.0 + LAG) - (1.0 - v) * LAG).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    const WINDOW: RectF = RectF {
        x: 100.0,
        y: 100.0,
        w: 400.0,
        h: 300.0,
    };
    const TILE: RectF = RectF {
        x: 900.0,
        y: 1040.0,
        w: 48.0,
        h: 30.0,
    };

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn progress_zero_is_the_window_and_one_is_the_tile() {
        // name, progress, (x, y, w, h) of the whole stack of strips
        type Rect = (f64, f64, f64, f64);
        const CASES: &[(&str, f64, Rect)] = &[
            ("rest", 0.0, (100.0, 100.0, 400.0, 300.0)),
            ("below zero", -1.0, (100.0, 100.0, 400.0, 300.0)),
            ("arrived", 1.0, (900.0, 1040.0, 48.0, 30.0)),
            ("past one", 2.0, (900.0, 1040.0, 48.0, 30.0)),
        ];
        for (name, progress, (x, y, w, h)) in CASES {
            let all = strips(WINDOW, TILE, *progress, STRIPS);
            let (first, last) = (all[0], all[STRIPS - 1]);
            assert!(close(first.dst.y, *y), "{name}: top {}", first.dst.y);
            assert!(close(last.dst.y + last.dst.h, y + h), "{name}: bottom");
            assert!(close(first.dst.x, *x), "{name}: left {}", first.dst.x);
            assert!(close(first.dst.w, *w), "{name}: width {}", first.dst.w);
            assert!(close(last.dst.x, *x), "{name}: last left");
        }
    }

    #[test]
    fn strips_share_their_edges_whatever_the_progress() {
        for step in 0..=20 {
            let progress = f64::from(step) / 20.0;
            let all = strips(WINDOW, TILE, progress, STRIPS);
            for pair in all.windows(2) {
                assert!(close(pair[0].dst.y + pair[0].dst.h, pair[1].dst.y));
                assert!(close(pair[0].bottom, pair[1].top));
            }
            assert!(all.iter().all(|strip| strip.dst.h >= 0.0), "p {progress}");
        }
    }

    #[test]
    fn the_bottom_narrows_first_along_a_curve() {
        let all = strips(WINDOW, TILE, 0.5, STRIPS);
        let widths: Vec<f64> = all.iter().map(|strip| strip.dst.w).collect();
        assert!(widths.windows(2).all(|pair| pair[1] <= pair[0] + 1e-9));
        let bottom = all[STRIPS - 1].dst;
        assert!(bottom.w < 90.0, "bottom is already narrow: {}", bottom.w);
        assert!(widths[0] > 300.0, "top is still wide: {}", widths[0]);
    }

    #[test]
    fn halfway_the_bottom_row_is_where_the_smooth_step_puts_it() {
        let all = strips(WINDOW, TILE, 0.5, 1);
        // One strip: rows v = 0 and v = 1. reach(0) = ease(0.2), reach(1) = ease(0.8).
        let ease = |t: f64| t * t * (3.0 - 2.0 * t);
        let (top, bottom) = (ease(0.2), ease(0.8));
        let width = (400.0 + (48.0 - 400.0) * top + 400.0 + (48.0 - 400.0) * bottom) / 2.0;
        assert!(close(all[0].dst.w, width), "{} vs {width}", all[0].dst.w);
        assert!(close(all[0].dst.y, 100.0 + (1040.0 - 100.0) * top));
    }

    #[test]
    fn a_tile_above_the_window_never_folds_it() {
        let above = RectF {
            x: 300.0,
            y: -50.0,
            w: 40.0,
            h: 20.0,
        };
        for step in 0..=20 {
            let all = strips(WINDOW, above, f64::from(step) / 20.0, STRIPS);
            assert!(all.iter().all(|strip| strip.dst.h >= 0.0));
            assert!(all.windows(2).all(|pair| pair[1].dst.y >= pair[0].dst.y));
        }
    }

    #[test]
    fn a_window_at_rest_is_cut_into_exactly_the_base_strips() {
        let all = strips_fine(WINDOW, TILE, 0.0, STRIPS, TOLERANCE, MAX_STRIPS);
        assert_eq!(all.len(), STRIPS);
        assert_eq!(all, strips(WINDOW, TILE, 0.0, STRIPS));
    }

    #[test]
    fn a_limit_of_the_base_count_is_the_plain_strips() {
        for step in 0..=10 {
            let progress = f64::from(step) / 10.0;
            let fine = strips_fine(WINDOW, TILE, progress, STRIPS, TOLERANCE, STRIPS);
            assert_eq!(fine, strips(WINDOW, TILE, progress, STRIPS), "p {progress}");
        }
    }

    #[test]
    fn the_steep_part_is_cut_finer_within_the_limit() {
        for step in 1..20 {
            let progress = f64::from(step) / 20.0;
            let all = strips_fine(WINDOW, TILE, progress, STRIPS, TOLERANCE, MAX_STRIPS);
            assert!(
                all.len() >= STRIPS && all.len() <= MAX_STRIPS,
                "p {progress}: {}",
                all.len()
            );
            for pair in all.windows(2) {
                assert!(
                    close(pair[0].dst.y + pair[0].dst.h, pair[1].dst.y),
                    "p {progress}"
                );
                assert!(close(pair[0].bottom, pair[1].top), "p {progress}");
            }
            assert!(close(all[0].top, 0.0) && close(all[all.len() - 1].bottom, 1.0));
        }
        let mid = strips_fine(WINDOW, TILE, 0.5, STRIPS, TOLERANCE, MAX_STRIPS);
        assert!(
            mid.len() > STRIPS,
            "halfway the curve is steep: {}",
            mid.len()
        );
    }

    #[test]
    fn with_room_enough_each_strip_edge_is_within_the_tolerance_of_its_rows() {
        let all = strips_fine(WINDOW, TILE, 0.5, STRIPS, 2.0, 100_000);
        for strip in &all {
            let row = |v: f64| {
                let went = reach(v, 0.5);
                let width = 400.0 + (48.0 - 400.0) * went;
                let centre = 300.0 + (924.0 - 300.0) * went;
                (centre - width / 2.0, centre + width / 2.0)
            };
            let (above, below) = (row(strip.top), row(strip.bottom));
            let left = strip.dst.x;
            let right = strip.dst.x + strip.dst.w;
            for (edge, ends) in [(left, (above.0, below.0)), (right, (above.1, below.1))] {
                let off = (edge - ends.0).abs().max((edge - ends.1).abs());
                // The cut is at equal heights, the curve is not linear in them: a little slack.
                assert!(off <= 2.5, "an edge is {off} px off its rows");
            }
        }
    }

    #[test]
    fn a_strip_with_no_whole_row_joins_the_next_and_a_last_one_the_one_before() {
        let strip = |top: f64, y: f64, h: f64, x: f64| Strip {
            top,
            bottom: top + 0.25,
            dst: RectF { x, y, w: 10.0, h },
        };
        let row = |y: f64| y.round() as i32;
        let joined = whole_rows(
            vec![
                strip(0.0, 0.0, 2.0, 0.0),
                strip(0.25, 2.0, 0.2, 1.0),
                strip(0.5, 2.2, 2.0, 2.0),
                strip(0.75, 4.2, 0.1, 3.0),
            ],
            row,
        );
        assert_eq!(joined.len(), 2);
        assert_eq!((joined[0].top, joined[0].bottom), (0.0, 0.25));
        assert_eq!((joined[1].top, joined[1].bottom), (0.25, 1.0));
        assert_eq!(joined[1].dst.x, 1.0);
        assert!((joined[1].dst.y + joined[1].dst.h - 4.3).abs() < 1e-9);
    }

    #[test]
    fn no_strips_asked_still_gives_one() {
        assert_eq!(strips(WINDOW, TILE, 0.3, 0).len(), 1);
    }
}
