//! The conversion tables: logical to device pixels at the four scales a compositor announces
//! most (1, 1.25, 1.5, 2), and the properties that make a layout tile at every one of them.

use super::*;
use ds_core::geometry::scale::Scale;
use proptest::prelude::*;

/// The scales of the tables, as 120ths.
const SCALES: [(&str, u32); 4] = [("1", 120), ("1.25", 150), ("1.5", 180), ("2", 240)];

fn rect(x: i32, y: i32, w: u32, h: u32) -> LogicalRect {
    LogicalRect::new(LogicalPoint { x, y }, LogicalSize { w, h })
}

fn phys(x: i32, y: i32, w: u32, h: u32) -> PhysicalRect {
    PhysicalRect { x, y, w, h }
}

/// A name, a logical rectangle, and its device rectangle at 1, 1.25, 1.5 and 2.
type RectRow = (&'static str, LogicalRect, [PhysicalRect; 4]);

#[test]
fn to_physical_at_the_four_scales() {
    let rows: [RectRow; 7] = [
        (
            "empty at the origin",
            rect(0, 0, 0, 0),
            [phys(0, 0, 0, 0); 4],
        ),
        (
            "one pixel",
            rect(0, 0, 1, 1),
            [
                phys(0, 0, 1, 1),
                phys(0, 0, 1, 1),
                phys(0, 0, 2, 2),
                phys(0, 0, 2, 2),
            ],
        ),
        (
            "one pixel at (1, 1)",
            rect(1, 1, 1, 1),
            [
                phys(1, 1, 1, 1),
                phys(1, 1, 2, 2),
                phys(2, 2, 1, 1),
                phys(2, 2, 2, 2),
            ],
        ),
        (
            "an odd window",
            rect(7, 9, 101, 51),
            [
                phys(7, 9, 101, 51),
                phys(9, 11, 126, 64),
                phys(11, 14, 151, 76),
                phys(14, 18, 202, 102),
            ],
        ),
        (
            "a full HD output",
            rect(0, 0, 1920, 1080),
            [
                phys(0, 0, 1920, 1080),
                phys(0, 0, 2400, 1350),
                phys(0, 0, 2880, 1620),
                phys(0, 0, 3840, 2160),
            ],
        ),
        (
            "a rectangle left of the origin",
            rect(-3, -1, 2, 2),
            [
                phys(-3, -1, 2, 2),
                phys(-4, -1, 3, 2),
                phys(-5, -2, 3, 4),
                phys(-6, -2, 4, 4),
            ],
        ),
        (
            "a second output to the right",
            rect(1920, 0, 1280, 720),
            [
                phys(1920, 0, 1280, 720),
                phys(2400, 0, 1600, 900),
                phys(2880, 0, 1920, 1080),
                phys(3840, 0, 2560, 1440),
            ],
        ),
    ];
    for (name, logical, want) in rows {
        for ((label, numerator), want) in SCALES.iter().zip(want) {
            assert_eq!(
                to_physical(logical, Scale(*numerator)),
                want,
                "{name} at {label}x"
            );
        }
    }
}

/// A name, a logical size, and its buffer at 1, 1.25, 1.5 and 2 (`round(l × s)` per side,
/// halves up).
type SizeRow = (&'static str, LogicalSize, [PhysicalSize; 4]);

fn size(w: u32, h: u32) -> LogicalSize {
    LogicalSize { w, h }
}

fn buf(w: u32, h: u32) -> PhysicalSize {
    PhysicalSize { w, h }
}

#[test]
fn buffer_size_is_round_l_times_s_at_the_four_scales() {
    let rows: [SizeRow; 6] = [
        ("zero", size(0, 0), [buf(0, 0); 4]),
        (
            "one",
            size(1, 1),
            [buf(1, 1), buf(1, 1), buf(2, 2), buf(2, 2)],
        ),
        (
            "x.25 and x.5 fractions",
            size(1001, 701),
            [
                buf(1001, 701),
                buf(1251, 876),
                buf(1502, 1052),
                buf(2002, 1402),
            ],
        ),
        (
            "x.75 fractions",
            size(103, 7),
            [buf(103, 7), buf(129, 9), buf(155, 11), buf(206, 14)],
        ),
        (
            "a dialog",
            size(480, 320),
            [buf(480, 320), buf(600, 400), buf(720, 480), buf(960, 640)],
        ),
        (
            "one pixel of margin",
            size(3, 5),
            [buf(3, 5), buf(4, 6), buf(5, 8), buf(6, 10)],
        ),
    ];
    for (name, logical, want) in rows {
        for ((label, numerator), want) in SCALES.iter().zip(want) {
            assert_eq!(
                buffer_size(logical, Scale(*numerator)),
                want,
                "{name} at {label}x"
            );
        }
    }
}

#[test]
fn extreme_values_saturate_instead_of_wrapping() {
    let huge = to_physical(rect(i32::MAX, i32::MIN, u32::MAX, u32::MAX), Scale(240));
    assert_eq!((huge.x, huge.y), (i32::MAX, i32::MIN));
    assert_eq!(
        buffer_size(size(u32::MAX, u32::MAX), Scale(240)),
        buf(u32::MAX, u32::MAX)
    );
    assert_eq!(buffer_size(size(5, 5), Scale(0)), buf(0, 0));
}

proptest! {
    /// Rectangles that touch in logical space touch in device space, at any scale.
    #[test]
    fn touching_rectangles_touch_at_every_scale(
        x in -5_000i32..5_000,
        y in -5_000i32..5_000,
        w1 in 0u32..3_000,
        w2 in 0u32..3_000,
        h in 0u32..3_000,
        numerator in 1u32..960,
    ) {
        let scale = Scale(numerator);
        let left = to_physical(rect(x, y, w1, h), scale);
        let right_origin = x + i32::try_from(w1).expect("small");
        let right = to_physical(rect(right_origin, y, w2, h), scale);
        prop_assert_eq!(i64::from(left.x) + i64::from(left.w), i64::from(right.x));
        prop_assert_eq!((left.y, left.h), (right.y, right.h));
    }

    /// A buffer is the physical size of a rectangle at the origin, and each side is within half
    /// a device pixel of the exact product.
    #[test]
    fn a_buffer_is_the_origin_rectangle_within_half_a_pixel(
        w in 0u32..100_000,
        h in 0u32..100_000,
        numerator in 1u32..960,
    ) {
        let scale = Scale(numerator);
        let buffer = buffer_size(size(w, h), scale);
        let origin = to_physical(rect(0, 0, w, h), scale);
        prop_assert_eq!((buffer.w, buffer.h), (origin.w, origin.h));
        let twice = |logical: u32, device: u32| {
            (2 * i64::from(logical) * i64::from(numerator) - 240 * i64::from(device)).abs()
        };
        prop_assert!(twice(w, buffer.w) <= 120);
        prop_assert!(twice(h, buffer.h) <= 120);
    }
}
