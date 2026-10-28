//! Turning a point or a size with an output's transform: where a point of an upright image is
//! on the frame the transform produced. One home for the rule, so a cursor's position, its
//! hotspot and its pixels all turn the same way.

use super::PhysicalSize;
use casement_protocols::vocab::Transform;

/// The point `(x, y)` of an upright area of `area` (`(width, height)`) as the area turned by
/// `transform` has it. For a pixel's index in a `w x h` image pass `(w - 1, h - 1)` as the
/// area.
///
/// ```
/// use casement_core::geometry::turn_point;
/// use casement_protocols::vocab::Transform;
///
/// // A quarter turn: the point 5 right and 3 down of a 24 x 24 image.
/// assert_eq!(turn_point(Transform::R90, (5, 3), (24, 24)), (21, 5));
/// assert_eq!(turn_point(Transform::Normal, (5, 3), (24, 24)), (5, 3));
/// ```
pub fn turn_point(transform: Transform, (x, y): (i32, i32), (w, h): (i32, i32)) -> (i32, i32) {
    match transform {
        Transform::Normal => (x, y),
        Transform::R90 => (h - y, x),
        Transform::R180 => (w - x, h - y),
        Transform::R270 => (y, w - x),
        Transform::Flipped => (w - x, y),
        Transform::Flipped90 => (y, x),
        Transform::Flipped180 => (x, h - y),
        Transform::Flipped270 => (h - y, w - x),
    }
}

/// The size of an area after `transform`: width and height swap for the quarter turns.
///
/// ```
/// use casement_core::geometry::{PhysicalSize, turn_size};
/// use casement_protocols::vocab::Transform;
///
/// let size = PhysicalSize { w: 48, h: 32 };
/// assert_eq!(turn_size(size, Transform::R270), PhysicalSize { w: 32, h: 48 });
/// assert_eq!(turn_size(size, Transform::Flipped), size);
/// ```
pub fn turn_size(size: PhysicalSize, transform: Transform) -> PhysicalSize {
    match transform {
        Transform::R90 | Transform::R270 | Transform::Flipped90 | Transform::Flipped270 => {
            PhysicalSize {
                w: size.h,
                h: size.w,
            }
        }
        _ => size,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Transform; 8] = [
        Transform::Normal,
        Transform::R90,
        Transform::R180,
        Transform::R270,
        Transform::Flipped,
        Transform::Flipped90,
        Transform::Flipped180,
        Transform::Flipped270,
    ];

    #[test]
    fn a_point_turns_inside_the_turned_area() {
        // The 6 x 4 area's corners land on corners of the turned area, whichever transform.
        for transform in ALL {
            let turned = turn_size(PhysicalSize { w: 6, h: 4 }, transform);
            let (tw, th) = (
                i32::try_from(turned.w).expect("small"),
                i32::try_from(turned.h).expect("small"),
            );
            for corner in [(0, 0), (6, 0), (0, 4), (6, 4)] {
                let (x, y) = turn_point(transform, corner, (6, 4));
                assert!(
                    (x == 0 || x == tw) && (y == 0 || y == th),
                    "{transform:?} {corner:?} -> {x},{y}"
                );
            }
        }
    }

    #[test]
    fn a_pixel_index_uses_the_area_less_one() {
        // Pixel (0, 0) of a 3 x 2 image under a quarter turn is the top-right of the 2 x 3 one.
        assert_eq!(turn_point(Transform::R90, (0, 0), (2, 1)), (1, 0));
        // Its bottom-right pixel (2, 1) is the bottom-left.
        assert_eq!(turn_point(Transform::R90, (2, 1), (2, 1)), (0, 2));
    }

    #[test]
    fn the_quarter_turns_swap_the_sides() {
        let size = PhysicalSize { w: 7, h: 3 };
        for transform in ALL {
            let swaps = matches!(
                transform,
                Transform::R90 | Transform::R270 | Transform::Flipped90 | Transform::Flipped270
            );
            let expected = if swaps {
                PhysicalSize { w: 3, h: 7 }
            } else {
                size
            };
            assert_eq!(turn_size(size, transform), expected, "{transform:?}");
        }
    }
}
