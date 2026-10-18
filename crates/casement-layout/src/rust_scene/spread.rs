//! The Mission Control spread (03 section 1.8; the recipe is in `rust_scene/mod.rs`): every
//! window of the shown Space shrunk by one common factor and laid out without overlap, in whole
//! numbers, below a strip the shell fills with the Spaces bar.
//!
//! A window's slot holds its content; a server-side frame reaches `reserve` logical pixels above
//! it at the window's own size, and the renderer draws the frame scaled with the window, so the
//! box laid out is the scaled content plus the scaled reserve. Below the content the cell keeps
//! the lower half of the app icon and the title that shows under it (as the Mac does), so an
//! icon never covers the window beneath.

use super::icon_rect::{below, icon_rect};
use super::spread_fit::{Cell, held, push_apart, shelf, start};
use crate::{LayoutKnobs, OverviewSlot, SlotKind};
use casement_core::WindowId;
use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize};

/// One part in a million: the scale's unit.
const WHOLE: i64 = 1_000_000;

/// The share of the area the windows (and their gaps) may fill at the start, in percent.
const DENSITY: i64 = 55;

/// A start scale this close to 1 (a fifth) is replaced by 1.
const NEAR_WHOLE: i64 = 800_000;

/// Each failed trial scales by 85 percent.
const FAILED_TRIAL: i64 = 85;

/// The most windows the pushing trials run for. The trials cost up to 24 passes of every pair
/// at about fourteen scales, so a Space with thousands of windows (a client that opens them in
/// a loop) would stall the compositor for seconds; above this the spread goes straight to the
/// shelf pack at the floor scale, which is linear after one sort.
const MAX_PUSHED: usize = 96;

/// The strip the Spaces bar takes at the top, in logical pixels (at most a quarter of the area).
const STRIP: u32 = 112;

/// A window to spread: where it is, and the frame above it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Entry {
    pub(super) window: WindowId,
    pub(super) rect: LogicalRect,
    pub(super) reserve: u32,
}

/// Whether the windows could be kept apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Fit {
    /// Every pair has at least the gap between it.
    Apart,
    /// Not even the floor scale separates them: overlaps remain, every slot is inside the area.
    Overflow,
}

/// The slots of `entries` inside `usable` (below the strip), and whether they fit.
pub(super) fn spread(
    entries: &[Entry],
    usable: LogicalRect,
    knobs: &LayoutKnobs,
) -> (Vec<OverviewSlot>, Fit) {
    if entries.is_empty() {
        return (Vec::new(), Fit::Apart);
    }
    let strip = STRIP.min(usable.size.h / 4);
    let area = Cell {
        x: i64::from(usable.origin.x),
        y: i64::from(usable.origin.y) + i64::from(strip),
        w: i64::from(usable.size.w),
        h: i64::from(usable.size.h - strip),
    };
    let ruler = Ruler::of(entries, knobs);
    let mut order: Vec<usize> = (0..entries.len()).collect();
    order.sort_by_key(|&i| {
        (
            entries[i].rect.origin.x,
            entries[i].rect.origin.y,
            entries[i].window.0,
        )
    });
    let centres: Vec<(i64, i64)> = entries.iter().map(centre).collect();
    let floor = ruler.floor;
    let (trials, mut at_floor) = match entries.len() > MAX_PUSHED {
        true => (Vec::new(), start(&centres, &ruler.sizes(floor), area)),
        false => (ruler.trials(area), Vec::new()),
    };
    for scale in trials {
        let sizes = ruler.sizes(scale);
        let mut cells = start(&centres, &sizes, area);
        if push_apart(&mut cells, area, &order) {
            return (ruler.slots(&cells, scale, usable), Fit::Apart);
        }
        at_floor = cells;
    }
    let sizes = ruler.sizes(floor);
    if let Some(cells) = shelf(&sizes, area, &order) {
        (ruler.slots(&cells, floor, usable), Fit::Apart)
    } else {
        held(&mut at_floor, area);
        (ruler.slots(&at_floor, floor, usable), Fit::Overflow)
    }
}

/// A window's original centre.
fn centre(entry: &Entry) -> (i64, i64) {
    let rect = entry.rect;
    (
        i64::from(rect.origin.x) + i64::from(rect.size.w) / 2,
        i64::from(rect.origin.y) + i64::from(rect.size.h) / 2,
    )
}

/// The windows' sizes and the numbers that turn a scale into boxes.
struct Ruler<'a> {
    entries: &'a [Entry],
    gap: i64,
    floor: i64,
}

impl<'a> Ruler<'a> {
    fn of(entries: &'a [Entry], knobs: &LayoutKnobs) -> Ruler<'a> {
        Ruler {
            entries,
            gap: i64::from(knobs.spread_gap),
            floor: (i64::from(knobs.spread_floor.0) * 1000).clamp(1, WHOLE),
        }
    }

    /// `side` scaled, never below one pixel.
    fn scaled(side: u32, scale: i64) -> i64 {
        (i64::from(side) * scale / WHOLE).max(1)
    }

    /// The frame's reserve at `scale`, rounded up so the scaled frame (which the renderer rounds
    /// to nearest) always fits the box.
    fn reserved(reserve: u32, scale: i64) -> i64 {
        (i64::from(reserve) * scale + WHOLE - 1) / WHOLE
    }

    /// Each cell's size at `scale`: the scaled content, the frame's reserve, the room below for
    /// the icon and its title, and the gap.
    fn sizes(&self, scale: i64) -> Vec<(i64, i64)> {
        self.entries
            .iter()
            .map(|e| {
                (
                    Self::scaled(e.rect.size.w, scale) + self.gap,
                    Self::scaled(e.rect.size.h, scale)
                        + Self::reserved(e.reserve, scale)
                        + below(scale)
                        + self.gap,
                )
            })
            .collect()
    }

    /// The scales to try, first to last: the closed-form start, each trial after a failure 85
    /// percent of the one before, the last the floor.
    fn trials(&self, area: Cell) -> Vec<i64> {
        let mut scale = self.start(area);
        let mut all = vec![scale];
        while scale > self.floor {
            scale = (scale * FAILED_TRIAL / 100).max(self.floor);
            all.push(scale);
        }
        all
    }

    /// Whether every cell at `scale` fits the area, and the sum of their areas.
    fn measure(&self, scale: i64, area: Cell) -> (bool, i64) {
        let sizes = self.sizes(scale);
        let fits = sizes.iter().all(|s| s.0 <= area.w && s.1 <= area.h);
        (fits, sizes.iter().map(|s| s.0 * s.1).sum())
    }

    /// The largest scale whose cells fit and fill at most [`DENSITY`] percent of the area,
    /// else the floor; 1 when that is within a fifth of it and the cells fit at 1.
    fn start(&self, area: Cell) -> i64 {
        let budget = area.w * area.h * DENSITY / 100;
        let ok = |scale: i64| {
            let (fits, used) = self.measure(scale, area);
            fits && used <= budget
        };
        let (mut low, mut high) = (self.floor, WHOLE);
        while low < high {
            let mid = low + (high - low + 1) / 2;
            match ok(mid) {
                true => low = mid,
                false => high = mid - 1,
            }
        }
        match (low >= NEAR_WHOLE, self.measure(WHOLE, area).0) {
            (true, true) => WHOLE,
            _ => low,
        }
    }

    /// The slots of the windows in `cells` at `scale`, held to `usable`.
    fn slots(&self, cells: &[Cell], scale: i64, usable: LogicalRect) -> Vec<OverviewSlot> {
        let half = self.gap / 2;
        self.entries
            .iter()
            .zip(cells)
            .map(|(e, cell)| {
                let size = LogicalSize {
                    w: u32::try_from(Self::scaled(e.rect.size.w, scale)).unwrap_or(1),
                    h: u32::try_from(Self::scaled(e.rect.size.h, scale)).unwrap_or(1),
                };
                let at = LogicalPoint {
                    x: whole(cell.x + half),
                    y: whole(cell.y + half + Self::reserved(e.reserve, scale)),
                };
                let rect = LogicalRect::new(at, size);
                let rect = rect.intersection(&usable).unwrap_or(rect);
                OverviewSlot {
                    window: e.window,
                    kind: SlotKind::Live,
                    rect,
                    icon: icon_rect(rect, scale, usable),
                }
            })
            .collect()
    }
}

/// `n` as a logical coordinate.
fn whole(n: i64) -> i32 {
    i32::try_from(n).unwrap_or(if n < 0 { i32::MIN } else { i32::MAX })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ds_core::vocab::Fraction;

    fn rect(x: i32, y: i32, w: u32, h: u32) -> LogicalRect {
        LogicalRect::new(LogicalPoint { x, y }, LogicalSize { w, h })
    }

    fn knobs() -> LayoutKnobs {
        LayoutKnobs {
            cascade_step: LogicalPoint { x: 28, y: 28 },
            tile_margin: 8,
            spread_gap: 24,
            spread_floor: Fraction(300),
        }
    }

    fn entry(n: u64, at: LogicalRect) -> Entry {
        Entry {
            window: WindowId(n),
            rect: at,
            reserve: 0,
        }
    }

    const USABLE: LogicalRect = LogicalRect {
        origin: LogicalPoint { x: 0, y: 24 },
        size: LogicalSize { w: 1872, h: 860 },
    };

    /// CASC-03: a Space with far more windows than the pushing trials can afford is still given
    /// a slot for every window, inside the usable area, without running the trials.
    #[test]
    fn a_crowd_of_windows_is_packed_without_the_trials() {
        let offset = |n: u64, modulo: u64, step: i32| i32::try_from(n % modulo).unwrap_or(0) * step;
        let entries: Vec<Entry> = (0..3000)
            .map(|n| {
                entry(
                    n,
                    rect(10 + offset(n, 50, 20), 30 + offset(n, 40, 15), 640, 400),
                )
            })
            .collect();
        let (slots, _) = spread(&entries, USABLE, &knobs());
        assert_eq!(slots.len(), entries.len());
        assert!(
            slots
                .iter()
                .all(|slot| USABLE.intersection(&slot.rect) == Some(slot.rect))
        );
    }

    #[test]
    fn nothing_to_spread_is_nothing() {
        assert_eq!(spread(&[], USABLE, &knobs()), (Vec::new(), Fit::Apart));
    }

    #[test]
    fn one_window_that_fits_keeps_its_size_below_the_strip() {
        let (slots, fit) = spread(&[entry(1, rect(300, 300, 640, 400))], USABLE, &knobs());
        assert_eq!(fit, Fit::Apart);
        assert_eq!(slots.len(), 1);
        assert_eq!(slots[0].rect.size, LogicalSize { w: 640, h: 400 });
        assert!(slots[0].rect.origin.y >= 24 + 112, "{:?}", slots[0].rect);
    }

    #[test]
    fn windows_keep_their_left_to_right_and_top_to_bottom_order() {
        let entries = [
            entry(1, rect(0, 0, 800, 600)),
            entry(2, rect(900, 0, 800, 600)),
            entry(3, rect(450, 400, 800, 600)),
        ];
        let (slots, fit) = spread(&entries, USABLE, &knobs());
        assert_eq!(fit, Fit::Apart);
        assert!(slots[0].rect.origin.x < slots[1].rect.origin.x);
        assert!(slots[0].rect.origin.y <= slots[2].rect.origin.y);
        assert!(slots[1].rect.origin.y <= slots[2].rect.origin.y);
    }

    #[test]
    fn a_frame_keeps_its_reserve_above_the_slot() {
        let framed = Entry {
            reserve: 28,
            ..entry(1, rect(100, 100, 400, 300))
        };
        let (slots, _) = spread(&[framed], USABLE, &knobs());
        assert!(
            slots[0].rect.origin.y >= 24 + 112 + 28,
            "{:?}",
            slots[0].rect
        );
    }

    #[test]
    fn the_frame_reserve_shrinks_with_the_window() {
        let framed = Entry {
            reserve: 28,
            ..entry(1, rect(0, 0, 1000, 600))
        };
        let entries = [framed];
        let ruler = Ruler::of(&entries, &knobs());
        // name, scale (millionths), the box's height: content + reserve + the room below + the gap of 24
        let cases = [
            ("whole", 1_000_000, 600 + 28 + 68 + 24),
            ("half", 500_000, 300 + 14 + 52 + 24),
            ("a third rounds up", 333_333, 199 + 10 + 52 + 24),
            ("a tenth", 100_000, 60 + 3 + 52 + 24),
        ];
        for (name, scale, want) in cases {
            assert_eq!(ruler.sizes(scale)[0].1, want, "{name}");
        }
    }

    #[test]
    fn a_shrunk_window_keeps_only_its_scaled_frame_above_the_slot() {
        // Too big for the area at its own size, so it shrinks; the frame above the slot is then
        // less than the full 28 px.
        let framed = Entry {
            reserve: 28,
            ..entry(1, rect(0, 0, 1800, 1400))
        };
        let (slots, _) = spread(&[framed], USABLE, &knobs());
        let scale_ppm = i64::from(slots[0].rect.size.w) * 1_000_000 / 1800;
        assert!(scale_ppm < 700_000, "{scale_ppm}");
        let bar = Ruler::reserved(28, scale_ppm);
        assert!((1..28).contains(&bar), "{bar}");
        assert!(
            i64::from(slots[0].rect.origin.y) >= 24 + 112 + bar,
            "{:?}",
            slots[0].rect
        );
    }

    #[test]
    fn a_cell_keeps_room_under_the_slot_for_the_icon_and_its_title() {
        let (slots, _) = spread(&[entry(1, rect(300, 300, 640, 400))], USABLE, &knobs());
        let s = &slots[0];
        let slot_bottom = s.rect.origin.y + 400;
        let title_bottom = i64::from(s.icon.origin.y) + i64::from(s.icon.size.h) + 30;
        assert!(
            title_bottom <= i64::from(USABLE.origin.y) + 860,
            "{title_bottom}"
        );
        assert!(s.icon.origin.y < slot_bottom + 6, "{:?}", s.icon);
    }

    #[test]
    fn a_crowd_shrinks_every_window_by_the_same_factor() {
        let entries: Vec<Entry> = (0..12)
            .map(|n| {
                let n32 = i32::try_from(n).unwrap_or(0);
                entry(n + 1, rect(40 * n32, 30 * n32, 800, 500))
            })
            .collect();
        let (slots, fit) = spread(&entries, USABLE, &knobs());
        assert_eq!(fit, Fit::Apart);
        assert!(slots.iter().all(|s| s.rect.size == slots[0].rect.size));
        assert!(slots[0].rect.size.w < 800);
    }

    #[test]
    fn more_than_the_floor_can_hold_is_reported() {
        let entries: Vec<Entry> = (0..60)
            .map(|n| entry(n + 1, rect(0, 0, 1000, 700)))
            .collect();
        let (slots, fit) = spread(&entries, USABLE, &knobs());
        assert_eq!(fit, Fit::Overflow);
        assert!(
            slots
                .iter()
                .all(|s| s.rect.intersection(&USABLE) == Some(s.rect))
        );
    }
}
