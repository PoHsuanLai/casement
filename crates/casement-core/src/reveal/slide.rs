//! Where a revealing surface is drawn and hit: slid toward its edge, or faded in place.

use super::{Look, RevealEdge};
use crate::geometry::{LogicalPoint, LogicalRect};
use ds_core::vocab::Fraction;

/// A surface as it is drawn now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Slid {
    /// Its rectangle, in global logical pixels. The pointer meets the surface here too.
    pub area: LogicalRect,
    /// Its opacity in thousandths.
    pub opacity: Fraction,
}

/// `area` (where the surface rests, inside `output`) when it is `visible` thousandths in. A slide
/// moves it toward its edge until it is wholly off the output; a cross-fade keeps it in place
/// and fades it.
///
/// ```
/// use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize};
/// use casement_core::reveal::{Look, RevealEdge, slid};
/// use ds_core::vocab::Fraction;
///
/// let size = |w, h| LogicalSize { w, h };
/// let output = LogicalRect::new(LogicalPoint { x: 0, y: 0 }, size(100, 100));
/// let bar = LogicalRect::new(LogicalPoint { x: 0, y: 0 }, size(100, 20));
/// let half = slid(RevealEdge::Top, Look::Slide, output, bar, Fraction(500));
/// assert_eq!(half.area.origin.y, -10);
/// ```
pub fn slid(
    edge: RevealEdge,
    look: Look,
    output: LogicalRect,
    area: LogicalRect,
    visible: Fraction,
) -> Slid {
    let visible = visible.clamped();
    match look {
        Look::CrossFade => Slid {
            area,
            opacity: visible,
        },
        Look::Slide => {
            let top = i64::from(area.origin.y) + i64::from(area.size.h);
            let out = top - i64::from(output.origin.y);
            let floor = i64::from(output.origin.y) + i64::from(output.size.h);
            let below = floor - i64::from(area.origin.y);
            let away = match edge {
                RevealEdge::Top => -out.max(0),
                RevealEdge::Bottom => below.max(0),
            };
            let gone = i64::from(Fraction(1000).0 - visible.0);
            let shift = away * gone / 1000;
            let y = i64::from(area.origin.y) + shift;
            Slid {
                area: LogicalRect::new(
                    LogicalPoint {
                        x: area.origin.x,
                        y: i32::try_from(y).unwrap_or(area.origin.y),
                    },
                    area.size,
                ),
                opacity: Fraction(1000),
            }
        }
    }
}
