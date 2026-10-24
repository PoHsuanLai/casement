//! The spread's placement arithmetic: whole numbers only, so the same windows always land in
//! the same places. A [`Cell`] is a window's box grown by half the gap on every side, so two
//! cells that do not overlap leave the whole gap between the windows.

/// A rectangle in whole logical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Cell {
    pub(super) x: i64,
    pub(super) y: i64,
    pub(super) w: i64,
    pub(super) h: i64,
}

impl Cell {
    #[cfg(test)]
    fn right(&self) -> i64 {
        self.x + self.w
    }

    #[cfg(test)]
    fn bottom(&self) -> i64 {
        self.y + self.h
    }

    /// The cell moved to stand inside `area` (pinned to its top-left when larger).
    fn inside(self, area: Cell) -> Cell {
        let fit = |at: i64, size: i64, lo: i64, room: i64| at.min(lo + room - size).max(lo);
        Cell {
            x: fit(self.x, self.w, area.x, area.w),
            y: fit(self.y, self.h, area.y, area.h),
            ..self
        }
    }
}

/// How far two cells overlap on each axis; `None` when they share no area.
#[cfg(test)]
fn overlap(a: &Cell, b: &Cell) -> Option<(i64, i64)> {
    let ox = a.right().min(b.right()) - a.x.max(b.x);
    let oy = a.bottom().min(b.bottom()) - a.y.max(b.y);
    (ox > 0 && oy > 0).then_some((ox, oy))
}

/// Whether any two cells overlap.
#[cfg(test)]
fn crowded(cells: &[Cell]) -> bool {
    cells
        .iter()
        .enumerate()
        .any(|(i, a)| cells[i + 1..].iter().any(|b| overlap(a, b).is_some()))
}

/// The windows in `order` (top to bottom, then left to right) dealt into `count` rows of
/// near-equal length, the earlier rows taking one more when they do not divide evenly; each
/// row then runs left to right by `across`, so a window on the left stays on the left.
pub(super) fn rows(order: &[usize], count: usize, across: &[i64]) -> Vec<Vec<usize>> {
    let count = count.clamp(1, order.len().max(1));
    let (each, extra) = (order.len() / count, order.len() % count);
    let mut rest = order;
    (0..count)
        .map(|r| {
            let take = each + usize::from(r < extra);
            let (row, after) = rest.split_at(take.min(rest.len()));
            rest = after;
            let mut row = row.to_vec();
            row.sort_by_key(|&i| (across[i], i));
            row
        })
        .collect()
}

/// The width of the widest row and the height of all of them, of cells sized `sizes`.
pub(super) fn extent(rows: &[Vec<usize>], sizes: &[(i64, i64)]) -> (i64, i64) {
    let width = |row: &Vec<usize>| row.iter().map(|&i| sizes[i].0).sum::<i64>();
    let height = |row: &Vec<usize>| row.iter().map(|&i| sizes[i].1).max().unwrap_or(0);
    (
        rows.iter().map(width).max().unwrap_or(0),
        rows.iter().map(height).sum(),
    )
}

/// The cells of `rows`, each row centred across `area` and the block of rows centred in it
/// (pinned to its top-left when larger), each cell centred down its row. The result is in the
/// cells' own order.
pub(super) fn placed(rows: &[Vec<usize>], sizes: &[(i64, i64)], area: Cell) -> Vec<Cell> {
    let mut cells = vec![
        Cell {
            x: 0,
            y: 0,
            w: 0,
            h: 0
        };
        sizes.len()
    ];
    let (_, total) = extent(rows, sizes);
    let mut y = area.y + ((area.h - total) / 2).max(0);
    for row in rows {
        let (width, height) = extent(std::slice::from_ref(row), sizes);
        let mut x = area.x + ((area.w - width) / 2).max(0);
        for &i in row {
            let (w, h) = sizes[i];
            cells[i] = Cell {
                x,
                y: y + (height - h) / 2,
                w,
                h,
            };
            x += w;
        }
        y += height;
    }
    cells
}

/// `cells` held inside `area`.
pub(super) fn held(cells: &mut [Cell], area: Cell) {
    for cell in cells {
        *cell = cell.inside(area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const AREA: Cell = Cell {
        x: 0,
        y: 0,
        w: 400,
        h: 300,
    };

    fn cell(x: i64, y: i64, w: i64, h: i64) -> Cell {
        Cell { x, y, w, h }
    }

    #[test]
    fn overlap_needs_area_on_both_axes() {
        let a = cell(0, 0, 100, 100);
        // name, other, want
        let cases = [
            ("apart", cell(100, 0, 50, 50), None),
            ("corner touch", cell(100, 100, 50, 50), None),
            ("inside", cell(10, 20, 30, 40), Some((30, 40))),
            ("partly", cell(60, 90, 100, 100), Some((40, 10))),
        ];
        for (name, other, want) in cases {
            assert_eq!(overlap(&a, &other), want, "{name}");
        }
    }

    #[test]
    fn a_cell_is_held_inside_the_area() {
        let cases = [
            ("left", cell(-50, 10, 100, 100), cell(0, 10, 100, 100)),
            (
                "bottom right",
                cell(350, 280, 100, 100),
                cell(300, 200, 100, 100),
            ),
            (
                "bigger than the area",
                cell(40, 40, 500, 400),
                cell(0, 0, 500, 400),
            ),
        ];
        for (name, from, want) in cases {
            assert_eq!(from.inside(AREA), want, "{name}");
        }
    }

    #[test]
    fn rows_are_dealt_top_to_bottom_and_run_left_to_right() {
        // Three windows top to bottom (0, 1, 2), across at 900, 100, 500.
        let across = [900, 100, 500];
        // name, row count, want
        let cases = [
            ("one row", 1, vec![vec![1, 2, 0]]),
            ("two rows, the first longer", 2, vec![vec![1, 0], vec![2]]),
            ("a row each", 3, vec![vec![0], vec![1], vec![2]]),
            ("more rows than windows", 5, vec![vec![0], vec![1], vec![2]]),
        ];
        for (name, count, want) in cases {
            assert_eq!(rows(&[0, 1, 2], count, &across), want, "{name}");
        }
    }

    #[test]
    fn placed_rows_are_centred_and_never_overlap() {
        let sizes = [(150, 100), (150, 80), (100, 100)];
        let cells = placed(&[vec![0, 1], vec![2]], &sizes, AREA);
        assert!(!crowded(&cells));
        // The block is 200 tall in 300: 50 above it. The first row is 300 wide in 400.
        assert_eq!(cells[0], cell(50, 50, 150, 100));
        assert_eq!(cells[1], cell(200, 60, 150, 80));
        assert_eq!(cells[2], cell(150, 150, 100, 100));
        assert_eq!(extent(&[vec![0, 1], vec![2]], &sizes), (300, 200));
    }
}
