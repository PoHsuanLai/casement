//! The four corners of a rectangle as the shadow tiles and the tile-preview plate draw them:
//! one [`Corner`] (the shell vocabulary's), the order they are kept in, and the axes each mirrors.

use casement_protocols::vocab::Corner;

/// Every corner, in the order tiles are kept.
pub(crate) const CORNERS: [Corner; 4] = [
    Corner::TopLeft,
    Corner::TopRight,
    Corner::BottomLeft,
    Corner::BottomRight,
];

/// Which way `corner`'s own axes point against the screen's: -1 where its quadrant is the
/// mirror image of the top-left one in that axis.
pub(crate) fn corner_sign(corner: Corner) -> (i32, i32) {
    match corner {
        Corner::TopLeft => (1, 1),
        Corner::TopRight => (-1, 1),
        Corner::BottomLeft => (1, -1),
        Corner::BottomRight => (-1, -1),
    }
}

/// `corner`'s index in a tile array (its place in [`CORNERS`]).
pub(crate) fn corner_index(corner: Corner) -> usize {
    CORNERS.iter().position(|c| *c == corner).unwrap_or(0)
}
