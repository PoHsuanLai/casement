//! The windows part-way into Mission Control: what the fingers show while they are still down.

use super::{Arrangement, WindowTarget};
use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize};
use ds_core::vocab::Fraction;

/// A full opening, in thousandths.
const OPEN: i64 = 1000;

impl Arrangement {
    /// This overview arrangement with each window `openness` of the way from its desktop
    /// rectangle to its overview slot (0 on the desktop, 1000 in the slot; a window with no slot
    /// stays on the desktop). The result holds the windows' places and the Spaces' origins and
    /// nothing else: its windows' rectangles are the poses to draw, so it has no slots left for
    /// motion to prefer over them.
    ///
    /// ```
    /// use casement_layout::Arrangement;
    /// use ds_core::vocab::Fraction;
    ///
    /// let none = Arrangement::default();
    /// assert_eq!(none.spread_at(Fraction(500)), Arrangement::default());
    /// ```
    pub fn spread_at(&self, openness: Fraction) -> Arrangement {
        let windows = self
            .windows
            .iter()
            .map(|target| {
                let slot = self
                    .overview
                    .iter()
                    .find(|slot| slot.window == target.window);
                WindowTarget {
                    rect: slot.map_or(target.rect, |slot| {
                        between(target.rect, slot.rect, openness)
                    }),
                    ..target.clone()
                }
            })
            .collect();
        Arrangement {
            windows,
            spaces: self.spaces.clone(),
            ..Arrangement::default()
        }
    }
}

/// `from` and `to` mixed: `openness` thousandths of the way to `to` on every edge.
fn between(from: LogicalRect, to: LogicalRect, openness: Fraction) -> LogicalRect {
    let t = i64::from(openness.0).min(OPEN);
    let mix = |a: i64, b: i64| a + (b - a) * t / OPEN;
    let whole = |n: i64| i32::try_from(n).unwrap_or(0);
    let side = |n: i64| u32::try_from(n.max(0)).unwrap_or(0);
    LogicalRect::new(
        LogicalPoint {
            x: whole(mix(i64::from(from.origin.x), i64::from(to.origin.x))),
            y: whole(mix(i64::from(from.origin.y), i64::from(to.origin.y))),
        },
        LogicalSize {
            w: side(mix(i64::from(from.size.w), i64::from(to.size.w))),
            h: side(mix(i64::from(from.size.h), i64::from(to.size.h))),
        },
    )
}
