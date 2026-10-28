//! Where the window shadow's pieces go: four corner tiles and four edge strips round the
//! window's outline, whatever its size. Pure geometry, so a table test pins it. The middle is
//! not drawn: the window's opaque body covers it.

use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize};
use casement_protocols::vocab::Corner;

/// How far the shadow reaches past the outline on each side, in logical pixels.
pub(crate) const MARGIN: u32 = 80;

/// How far an edge strip reaches under the window's edge, in logical pixels: enough to meet the
/// anti-aliased edge of the window and its rounded corners without a gap.
pub(crate) const EDGE_DEPTH: u32 = 2;

/// A side of the outline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Side {
    Top,
    Bottom,
    Left,
    Right,
}

impl Side {
    /// Every side, in the order the strips are kept.
    pub(crate) const ALL: [Side; 4] = [Side::Top, Side::Bottom, Side::Left, Side::Right];

    /// The index in a strip array.
    pub(crate) fn index(self) -> usize {
        Side::ALL.iter().position(|s| *s == self).unwrap_or(0)
    }
}

/// One piece of the shadow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Piece {
    Corner(Corner),
    Edge(Side),
}

/// One piece placed: where it goes and which part of its tile it shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Slice {
    /// The piece.
    pub(crate) piece: Piece,
    /// Where it goes, in logical pixels from the outline's top-left corner.
    pub(crate) dest: LogicalRect,
    /// The part of the tile it shows, in the tile's own logical units (see [`tile_size`]).
    pub(crate) src: LogicalRect,
}

/// A tile's size in logical units: a corner is `MARGIN + inner` square; a top or bottom strip is
/// one unit wide and `MARGIN + EDGE_DEPTH` tall; a left or right strip is the other way round.
/// The pixels per unit differ by tile (a strip is one pixel along its axis).
pub(crate) fn tile_size(piece: Piece, inner: u32) -> LogicalSize {
    let strip = MARGIN + EDGE_DEPTH;
    match piece {
        Piece::Corner(_) => LogicalSize {
            w: MARGIN + inner,
            h: MARGIN + inner,
        },
        Piece::Edge(Side::Top | Side::Bottom) => LogicalSize { w: 1, h: strip },
        Piece::Edge(Side::Left | Side::Right) => LogicalSize { w: strip, h: 1 },
    }
}

fn rect(x: i64, y: i64, w: i64, h: i64) -> LogicalRect {
    let clamp = |v: i64| i32::try_from(v).unwrap_or(i32::MAX);
    let side = |v: i64| u32::try_from(v.max(0)).unwrap_or(u32::MAX);
    LogicalRect::new(
        LogicalPoint {
            x: clamp(x),
            y: clamp(y),
        },
        LogicalSize {
            w: side(w),
            h: side(h),
        },
    )
}

/// The pieces that draw the shadow of an outline `size` whose corner tiles reach `inner` logical
/// pixels into the window. A window smaller than twice `inner` crops its corners to half the
/// window (the shadow of a window that small is a little stronger than a true blur's); the
/// pieces never overlap. A strip is left out where the corners meet.
pub(crate) fn slices(size: LogicalSize, inner: u32) -> Vec<Slice> {
    let (w, h) = (i64::from(size.w), i64::from(size.h));
    let (m, inner) = (i64::from(MARGIN), i64::from(inner));
    let edge = i64::from(EDGE_DEPTH);
    let (kx, ky) = (inner.min(w / 2), inner.min(h / 2));
    let (ex, ey) = (edge.min(w / 2), edge.min(h / 2));
    let corner = |c: Corner, x: i64, y: i64, sx: i64, sy: i64, kx: i64, ky: i64| Slice {
        piece: Piece::Corner(c),
        dest: rect(x, y, kx + m, ky + m),
        src: rect(sx, sy, kx + m, ky + m),
    };
    let mut all = vec![
        corner(Corner::TopLeft, -m, -m, 0, 0, kx, ky),
        corner(Corner::TopRight, w - kx, -m, inner - kx, 0, kx, ky),
        corner(Corner::BottomLeft, -m, h - ky, 0, inner - ky, kx, ky),
        corner(
            Corner::BottomRight,
            w - kx,
            h - ky,
            inner - kx,
            inner - ky,
            kx,
            ky,
        ),
    ];
    let across = w - 2 * kx;
    if across > 0 {
        all.push(Slice {
            piece: Piece::Edge(Side::Top),
            dest: rect(kx, -m, across, m + ey),
            src: rect(0, 0, 1, m + ey),
        });
        all.push(Slice {
            piece: Piece::Edge(Side::Bottom),
            dest: rect(kx, h - ey, across, m + ey),
            src: rect(0, edge - ey, 1, m + ey),
        });
    }
    let down = h - 2 * ky;
    if down > 0 {
        all.push(Slice {
            piece: Piece::Edge(Side::Left),
            dest: rect(-m, ky, m + ex, down),
            src: rect(0, 0, m + ex, 1),
        });
        all.push(Slice {
            piece: Piece::Edge(Side::Right),
            dest: rect(w - ex, ky, m + ex, down),
            src: rect(edge - ex, 0, m + ex, 1),
        });
    }
    all
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(r: &LogicalRect) -> u64 {
        u64::from(r.size.w) * u64::from(r.size.h)
    }

    fn overlap(a: &LogicalRect, b: &LogicalRect) -> bool {
        a.intersection(b).is_some()
    }

    #[test]
    fn the_pieces_never_overlap_and_stay_in_their_tiles() {
        for (w, h, inner) in [
            (400, 300, 70),
            (1000, 40, 70),
            (50, 50, 70),
            (141, 141, 70),
            (140, 300, 70),
            (3, 200, 70),
            (1, 1, 70),
        ] {
            let size = LogicalSize { w, h };
            let all = slices(size, inner);
            for (i, a) in all.iter().enumerate() {
                for b in &all[i + 1..] {
                    assert!(!overlap(&a.dest, &b.dest), "{w}x{h}: {a:?} {b:?}");
                }
                let tile = tile_size(a.piece, inner);
                let far = (
                    i64::from(a.src.origin.x) + i64::from(a.src.size.w),
                    i64::from(a.src.origin.y) + i64::from(a.src.size.h),
                );
                assert!(
                    a.src.origin.x >= 0
                        && a.src.origin.y >= 0
                        && far.0 <= i64::from(tile.w)
                        && far.1 <= i64::from(tile.h),
                    "{w}x{h}: {a:?} leaves its tile"
                );
            }
        }
    }

    #[test]
    fn a_large_window_gets_eight_pieces_that_ring_the_outline() {
        let all = slices(LogicalSize { w: 400, h: 300 }, 70);
        assert_eq!(all.len(), 8);
        let ring: u64 = all.iter().map(|s| area(&s.dest)).sum();
        let (m, e, k) = (u64::from(MARGIN), u64::from(EDGE_DEPTH), 70);
        let corners = 4 * (m + k) * (m + k);
        let strips = 2 * (400 - 2 * k) * (m + e) + 2 * (300 - 2 * k) * (m + e);
        assert_eq!(ring, corners + strips);
    }

    #[test]
    fn the_size_of_the_window_changes_only_the_strips_dest() {
        let small = slices(LogicalSize { w: 400, h: 300 }, 70);
        let large = slices(LogicalSize { w: 1400, h: 900 }, 70);
        for (a, b) in small.iter().zip(&large) {
            assert_eq!(a.piece, b.piece);
            assert_eq!(a.src.size, b.src.size, "{:?}", a.piece);
        }
    }

    #[test]
    fn a_tiny_window_drops_the_strips_the_corners_have_taken() {
        let all = slices(LogicalSize { w: 100, h: 100 }, 70);
        assert_eq!(all.len(), 4);
        let odd = slices(LogicalSize { w: 101, h: 100 }, 70);
        assert!(odd.iter().any(|s| s.piece == Piece::Edge(Side::Top)));
    }

    #[test]
    fn the_corners_are_mirror_images() {
        let all = slices(LogicalSize { w: 400, h: 300 }, 70);
        let at = |c| {
            all.iter()
                .find(|s| s.piece == Piece::Corner(c))
                .map(|s| s.dest)
        };
        let (tl, tr) = (at(Corner::TopLeft), at(Corner::TopRight));
        let (bl, br) = (at(Corner::BottomLeft), at(Corner::BottomRight));
        assert_eq!(tl.map(|r| r.size), tr.map(|r| r.size));
        assert_eq!(bl.map(|r| r.size), br.map(|r| r.size));
        assert_eq!(tl.map(|r| r.origin.x), Some(-(MARGIN as i32)));
        assert_eq!(
            tr.map(|r| r.origin.x + r.size.w as i32),
            Some(400 + MARGIN as i32)
        );
        assert_eq!(
            br.map(|r| r.origin.y + r.size.h as i32),
            Some(300 + MARGIN as i32)
        );
    }
}
