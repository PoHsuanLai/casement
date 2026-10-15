//! The strip between the halves of a Split View, which the person drags.

use casement_core::geometry::{LogicalPoint, LogicalRect};
use casement_core::space::SplitPair;
use casement_core::{OutputKey, WindowId};
use ds_core::vocab::Fraction;
use porter_core::SpaceId;

/// A Split View Space's divider: where the strip is, and what a pointer position means for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DividerTarget {
    /// The Split View Space.
    pub space: SpaceId,
    /// The output it is on.
    pub output: OutputKey,
    /// The leading (left) window: the one `split_divide` names.
    pub leading: WindowId,
    /// The trailing (right) window.
    pub trailing: WindowId,
    /// The strip itself (the gap between the halves), in global logical coordinates.
    pub strip: LogicalRect,
    /// The row the halves and the strip share, in global logical coordinates.
    pub span: LogicalRect,
    /// The leading window's minimum width.
    pub min_leading: u32,
    /// The trailing window's minimum width.
    pub min_trailing: u32,
}

impl DividerTarget {
    /// Whether `at` is on the strip.
    ///
    /// ```
    /// use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize};
    /// use casement_core::{OutputKey, OutputName, WindowId};
    /// use casement_core::output::{EdidFacts, EdidSerial};
    /// use casement_layout::DividerTarget;
    /// use porter_core::SpaceId;
    ///
    /// let edid = EdidFacts { make: String::new(), model: String::new(), serial: EdidSerial::Unknown };
    /// let strip = LogicalRect::new(LogicalPoint { x: 956, y: 0 }, LogicalSize { w: 8, h: 1080 });
    /// let divider = DividerTarget {
    ///     space: SpaceId::parse("s-1").unwrap(),
    ///     output: OutputKey::of(&edid, &OutputName::parse("HEADLESS-1").unwrap()),
    ///     leading: WindowId(1),
    ///     trailing: WindowId(2),
    ///     strip,
    ///     span: LogicalRect::new(LogicalPoint { x: 0, y: 0 }, LogicalSize { w: 1920, h: 1080 }),
    ///     min_leading: 0,
    ///     min_trailing: 0,
    /// };
    /// assert!(divider.hit(LogicalPoint { x: 960, y: 500 }));
    /// assert!(!divider.hit(LogicalPoint { x: 955, y: 500 }));
    /// ```
    pub fn hit(&self, at: LogicalPoint) -> bool {
        self.strip.contains(at)
    }

    /// The leading window's share, in thousandths of the row without the strip, when the
    /// strip's centre is at pointer column `x`. It is kept within [`SplitPair::NEAREST`] and,
    /// when both minimum widths fit inside that, within what leaves each window its minimum.
    pub fn share_at(&self, x: i32) -> Fraction {
        let room = i64::from(self.span.size.w.saturating_sub(self.strip.size.w));
        if room == 0 {
            return Fraction(SplitPair::NEAREST.0);
        }
        let centre = i64::from(self.strip.size.w) / 2;
        let lead = i64::from(x) - i64::from(self.span.origin.x) - centre;
        let thousandths = |width: i64| (width * 1000 + room / 2) / room;
        let want = thousandths(lead);
        let (mut low, mut high) = (
            i64::from(SplitPair::NEAREST.0),
            i64::from(SplitPair::NEAREST.1),
        );
        let least = (i64::from(self.min_leading) * 1000 + room - 1) / room;
        let most = (room - i64::from(self.min_trailing)) * 1000 / room;
        if least.max(low) <= most.min(high) {
            low = low.max(least);
            high = high.min(most);
        }
        Fraction(u16::try_from(want.clamp(low, high)).unwrap_or(SplitPair::NEAREST.0))
    }
}
