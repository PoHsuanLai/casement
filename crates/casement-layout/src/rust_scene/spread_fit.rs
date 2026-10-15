//! The spread's placement arithmetic: whole numbers only, so the same windows always land in
//! the same places. A [`Cell`] is a window's box grown by half the gap on every side, so two
//! cells that do not overlap leave the whole gap between the windows.

/// How many pushing passes one trial gets (21-FREEZE-AMENDMENTS A21).
const PASSES: usize = 24;

/// A rectangle in whole logical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Cell {
    pub(super) x: i64,
    pub(super) y: i64,
    pub(super) w: i64,
    pub(super) h: i64,
}

impl Cell {
    fn right(&self) -> i64 {
        self.x + self.w
    }

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
fn overlap(a: &Cell, b: &Cell) -> Option<(i64, i64)> {
    let ox = a.right().min(b.right()) - a.x.max(b.x);
    let oy = a.bottom().min(b.bottom()) - a.y.max(b.y);
    (ox > 0 && oy > 0).then_some((ox, oy))
}

/// Whether any two cells overlap.
fn crowded(cells: &[Cell]) -> bool {
    cells
        .iter()
        .enumerate()
        .any(|(i, a)| cells[i + 1..].iter().any(|b| overlap(a, b).is_some()))
}

/// Where each cell starts: its window's centre mapped monotonically, per axis, from the span of
/// the original centres onto the span the area leaves for centres. `centres` are the windows'
/// original centres; `sizes` the cells' sizes.
pub(super) fn start(centres: &[(i64, i64)], sizes: &[(i64, i64)], area: Cell) -> Vec<Cell> {
    let widest = sizes.iter().map(|s| s.0).max().unwrap_or(0);
    let tallest = sizes.iter().map(|s| s.1).max().unwrap_or(0);
    let map = |pick: fn(&(i64, i64)) -> i64, lo: i64, room: i64, biggest: i64| {
        let (from, to) = (lo + biggest / 2, lo + room - biggest / 2);
        let values: Vec<i64> = centres.iter().map(pick).collect();
        let (low, high) = (
            values.iter().copied().min().unwrap_or(0),
            values.iter().copied().max().unwrap_or(0),
        );
        values
            .into_iter()
            .map(|c| match (high > low, to > from) {
                (true, true) => from + (c - low) * (to - from) / (high - low),
                _ => lo + room / 2,
            })
            .collect::<Vec<i64>>()
    };
    let xs = map(|c| c.0, area.x, area.w, widest);
    let ys = map(|c| c.1, area.y, area.h, tallest);
    xs.into_iter()
        .zip(ys)
        .zip(sizes)
        .map(|((cx, cy), size)| {
            Cell {
                x: cx - size.0 / 2,
                y: cy - size.1 / 2,
                w: size.0,
                h: size.1,
            }
            .inside(area)
        })
        .collect()
}

/// Pushes overlapping cells apart, pairs in `order`, each pair along the axis it overlaps least
/// (half the overlap each way, the leading one first), keeping every cell in `area`. Whether
/// the cells are separated after at most [`PASSES`] passes.
pub(super) fn push_apart(cells: &mut [Cell], area: Cell, order: &[usize]) -> bool {
    for _ in 0..PASSES {
        let mut moved = false;
        for (n, &i) in order.iter().enumerate() {
            for &j in &order[n + 1..] {
                let Some((ox, oy)) = overlap(&cells[i], &cells[j]) else {
                    continue;
                };
                moved = true;
                let along_x = ox <= oy;
                let centre = |c: &Cell| match along_x {
                    true => 2 * c.x + c.w,
                    false => 2 * c.y + c.h,
                };
                let (first, second) = match centre(&cells[i]) <= centre(&cells[j]) {
                    true => (i, j),
                    false => (j, i),
                };
                let by = if along_x { ox } else { oy };
                let (back, forward) = (by / 2, by - by / 2);
                let step = |cell: &mut Cell, d: i64| match along_x {
                    true => cell.x += d,
                    false => cell.y += d,
                };
                step(&mut cells[first], -back);
                step(&mut cells[second], forward);
                cells[first] = cells[first].inside(area);
                cells[second] = cells[second].inside(area);
            }
        }
        if !moved {
            return true;
        }
    }
    !crowded(cells)
}

/// The cells in rows, tallest first (ties by `order`), left to right, each row and the block
/// centred; `None` when the rows do not fit `area`. The result is in the cells' own order.
pub(super) fn shelf(sizes: &[(i64, i64)], area: Cell, order: &[usize]) -> Option<Vec<Cell>> {
    let mut tall: Vec<usize> = order.to_vec();
    tall.sort_by_key(|&i| std::cmp::Reverse(sizes[i].1));
    let mut rows: Vec<Vec<usize>> = Vec::new();
    let mut used = 0;
    for i in tall {
        let fits = !rows.is_empty() && used + sizes[i].0 <= area.w;
        if fits {
            used += sizes[i].0;
            if let Some(row) = rows.last_mut() {
                row.push(i);
            }
        } else {
            used = sizes[i].0;
            rows.push(vec![i]);
        }
    }
    let row_h = |row: &Vec<usize>| row.iter().map(|&i| sizes[i].1).max().unwrap_or(0);
    let total: i64 = rows.iter().map(row_h).sum();
    let too_wide = sizes.iter().any(|s| s.0 > area.w);
    if total > area.h || too_wide {
        return None;
    }
    let mut cells = vec![
        Cell {
            x: 0,
            y: 0,
            w: 0,
            h: 0
        };
        sizes.len()
    ];
    let mut y = area.y + (area.h - total) / 2;
    for row in &rows {
        let width: i64 = row.iter().map(|&i| sizes[i].0).sum();
        let mut x = area.x + (area.w - width) / 2;
        for &i in row {
            cells[i] = Cell {
                x,
                y,
                w: sizes[i].0,
                h: sizes[i].1,
            };
            x += sizes[i].0;
        }
        y += row_h(row);
    }
    Some(cells)
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
    fn two_stacked_cells_are_pushed_apart_along_their_smaller_overlap() {
        let mut cells = [cell(100, 100, 120, 80), cell(110, 100, 120, 80)];
        assert!(push_apart(&mut cells, AREA, &[0, 1]));
        assert!(!crowded(&cells));
        assert!(cells[0].x < cells[1].x, "the leading cell stays leading");
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
    fn the_shelf_fills_rows_tallest_first_and_refuses_what_cannot_fit() {
        let sizes = [(150, 100), (150, 100), (150, 100), (150, 100)];
        let cells = shelf(&sizes, AREA, &[0, 1, 2, 3]).expect("two rows of two");
        assert!(!crowded(&cells));
        assert_eq!(cells[0].y, cells[1].y);
        assert!(cells[2].y > cells[0].y);
        assert_eq!(shelf(&[(500, 10)], AREA, &[0]), None);
        assert_eq!(shelf(&[(300, 200), (300, 200)], AREA, &[0, 1]), None);
    }

    #[test]
    fn centres_keep_their_order_when_mapped_onto_the_area() {
        let centres = [(10, 10), (500, 20), (900, 400)];
        let sizes = [(100, 80); 3];
        let cells = start(&centres, &sizes, AREA);
        assert!(cells[0].x < cells[1].x && cells[1].x < cells[2].x);
        assert!(cells[0].y <= cells[1].y && cells[1].y < cells[2].y);
        assert!(cells.iter().all(|c| c.inside(AREA) == *c));
    }
}
