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

/// How many strips a window is cut into.
pub const STRIPS: usize = 48;

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
    let progress = progress.clamp(0.0, 1.0);
    let share = |index: usize| index as f64 / count as f64;
    let mut floor = f64::NEG_INFINITY;
    let rows: Vec<Row> = (0..=count)
        .map(|index| {
            let row = row_at(window, tile, progress, share(index));
            // A tile above the window's rows would fold the window over itself; rows never go up.
            floor = floor.max(row.y);
            Row { y: floor, ..row }
        })
        .collect();
    rows.windows(2)
        .zip(0..count)
        .map(|(pair, index)| {
            let (above, below) = (pair[0], pair[1]);
            let width = (above.width + below.width) / 2.0;
            let centre = (above.centre + below.centre) / 2.0;
            Strip {
                top: share(index),
                bottom: share(index + 1),
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
        const CASES: &[(&str, f64, (f64, f64, f64, f64))] = &[
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
    fn no_strips_asked_still_gives_one() {
        assert_eq!(strips(WINDOW, TILE, 0.3, 0).len(), 1);
    }
}
