//! The row of Spaces as one strip: each window drawn in its Space's column, the row shifted
//! by where the fingers (or the finish after them) have put it.
//!
//! Layout arranges every window as if its Space were shown, at the usable origin; this is the
//! one place that moves a window to its Space's column, so a Space beside the shown one has its
//! windows beside it while the row moves and out of sight when it rests. Pure sub-pixel
//! arithmetic over the sampled frame.

use casement_core::WindowId;
use casement_core::geometry::{LogicalRect, SubPx};
use casement_motion::PlacedWindow;
use std::collections::BTreeMap;

/// How far a neighbour's shadow reaches into view, in sub-pixels (160 logical px, the widest
/// shadow the renderer draws): a window whose body is farther out than this is not drawn.
const REACH: i64 = 160 * 120;

/// One output's row of Spaces at one instant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowDraw {
    /// How far the first Space is from where it would be if it were shown: 0 while the first
    /// Space is shown, minus one page for the second.
    pub offset: SubPx,
    /// One Space's width (the usable width), in sub-pixels.
    pub page: SubPx,
    /// The column of each window that lives on a Space; a window not here is on every Space and
    /// stays where it is.
    pub columns: BTreeMap<WindowId, u32>,
}

impl RowDraw {
    /// How far `window` is shifted along the row.
    ///
    /// ```
    /// use casement_core::WindowId;
    /// use casement_core::geometry::SubPx;
    /// use casement_render::RowDraw;
    ///
    /// let row = RowDraw {
    ///     offset: SubPx(-1_000),
    ///     page: SubPx(1_000),
    ///     columns: [(WindowId(1), 0), (WindowId(2), 1)].into(),
    /// };
    /// assert_eq!(row.shift_of(WindowId(1)), SubPx(-1_000));
    /// assert_eq!(row.shift_of(WindowId(2)), SubPx(0), "the shown Space sits in place");
    /// assert_eq!(row.shift_of(WindowId(3)), SubPx(0), "on every Space: not moved");
    /// ```
    pub fn shift_of(&self, window: WindowId) -> SubPx {
        let Some(column) = self.columns.get(&window) else {
            return SubPx(0);
        };
        let shift = i64::from(self.offset.0) + i64::from(*column) * i64::from(self.page.0);
        SubPx(i32::try_from(shift).unwrap_or(if shift < 0 { i32::MIN } else { i32::MAX }))
    }

    /// `windows` moved to their columns, those left out of sight of `area` (the output's
    /// rectangle) dropped; a window whose shadow only reaches in is kept. Their order is kept.
    pub fn shifted(&self, windows: &[PlacedWindow], area: LogicalRect) -> Vec<PlacedWindow> {
        self.placed_in(windows, area, REACH)
    }

    /// `windows` moved to their columns, those whose body is not on `area` dropped: what the
    /// pointer can reach.
    pub fn touching(&self, windows: &[PlacedWindow], area: LogicalRect) -> Vec<PlacedWindow> {
        self.placed_in(windows, area, 0)
    }

    fn placed_in(
        &self,
        windows: &[PlacedWindow],
        area: LogicalRect,
        reach: i64,
    ) -> Vec<PlacedWindow> {
        windows
            .iter()
            .filter_map(|placed| {
                let x = self.kept_x(placed.window, placed.at.x, placed.size.w, area, reach)?;
                let mut moved = *placed;
                moved.at.x = x;
                Some(moved)
            })
            .collect()
    }

    /// The left edge of a window at `x`, `w` wide, once the row moved it, or none when the row
    /// moved it out of reach of `area`. A window the row does not move stays, wherever it is.
    fn kept_x(
        &self,
        window: WindowId,
        x: SubPx,
        w: SubPx,
        area: LogicalRect,
        reach: i64,
    ) -> Option<SubPx> {
        let shift = self.shift_of(window);
        let moved = SubPx(x.0.saturating_add(shift.0));
        (shift.0 == 0 || reaches(moved, w, area, reach)).then_some(moved)
    }

    /// The windows of `columns` whose body or shadow touches `area` once shifted, as `(window,
    /// left edge)` for a window at `x` and `w` wide each.
    ///
    /// ```
    /// use casement_core::WindowId;
    /// use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize, SubPx};
    /// use casement_render::RowDraw;
    ///
    /// let area = LogicalRect::new(LogicalPoint { x: 0, y: 0 }, LogicalSize { w: 1000, h: 600 });
    /// let page = SubPx(120_000);
    /// let row = RowDraw {
    ///     offset: SubPx(-120_000),
    ///     page,
    ///     columns: [(WindowId(1), 0), (WindowId(2), 1), (WindowId(3), 2)].into(),
    /// };
    /// let seen = row.seen([(WindowId(1), SubPx(60_000), SubPx(6_000)),
    ///                      (WindowId(2), SubPx(60_000), SubPx(6_000)),
    ///                      (WindowId(3), SubPx(60_000), SubPx(6_000))], area);
    /// assert_eq!(seen, vec![(WindowId(2), SubPx(60_000))], "only the shown Space");
    /// ```
    pub fn seen(
        &self,
        windows: impl IntoIterator<Item = (WindowId, SubPx, SubPx)>,
        area: LogicalRect,
    ) -> Vec<(WindowId, SubPx)> {
        windows
            .into_iter()
            .filter_map(|(window, x, w)| Some((window, self.kept_x(window, x, w, area, REACH)?)))
            .collect()
    }
}

/// Whether a body from `left`, `width` wide, touches `area` along the row, or comes within
/// `margin` sub-pixels of it.
fn reaches(left: SubPx, width: SubPx, area: LogicalRect, margin: i64) -> bool {
    let left = i64::from(left.0);
    let right = left + i64::from(width.0);
    let from = i64::from(SubPx::of_logical(area.origin.x).0);
    let across = i32::try_from(area.size.w).unwrap_or(i32::MAX);
    let to = i64::from(SubPx::of_logical(area.origin.x.saturating_add(across)).0);
    right > from - margin && left < to + margin
}

#[cfg(test)]
mod tests {
    use super::*;
    use casement_core::geometry::{LogicalPoint, LogicalSize};

    const PAGE: i32 = 1000 * 120;

    fn area() -> LogicalRect {
        LogicalRect::new(LogicalPoint { x: 0, y: 0 }, LogicalSize { w: 1000, h: 600 })
    }

    fn row(offset: i32) -> RowDraw {
        RowDraw {
            offset: SubPx(offset),
            page: SubPx(PAGE),
            columns: [(WindowId(1), 0), (WindowId(2), 1), (WindowId(3), 2)].into(),
        }
    }

    /// Windows 1, 2 and 3, each 50 logical px wide and 500 px in, as layout arranges them.
    fn frame() -> Vec<(WindowId, SubPx, SubPx)> {
        (1..=3)
            .map(|n| (WindowId(n), SubPx(60_000), SubPx(6_000)))
            .collect()
    }

    #[test]
    fn the_row_rests_with_the_shown_space_in_place_and_the_others_out_of_sight() {
        let seen = row(-PAGE).seen(frame(), area());
        assert_eq!(seen, vec![(WindowId(2), SubPx(60_000))]);
    }

    #[test]
    fn half_way_between_two_spaces_both_show() {
        let seen = row(-PAGE / 2).seen(frame(), area());
        assert_eq!(
            seen,
            vec![
                (WindowId(1), SubPx(60_000 - PAGE / 2)),
                (WindowId(2), SubPx(60_000 + PAGE / 2)),
            ]
        );
    }

    #[test]
    fn a_window_on_every_space_stays_put_and_the_rubber_band_shows_the_edge() {
        let stay = [(WindowId(9), SubPx(-6_000), SubPx(12_000))];
        assert_eq!(
            row(-PAGE).seen(stay, area()),
            vec![(WindowId(9), SubPx(-6_000))]
        );
        // Dragged a quarter page past the first Space: it sits a quarter page right.
        let seen = row(PAGE / 4).seen(frame(), area());
        assert_eq!(seen, vec![(WindowId(1), SubPx(60_000 + PAGE / 4))]);
    }
}
