//! The edge: smithay's types become ours here and in the handlers, never deeper (CONVENTIONS
//! addendum 3).

use casement_core::InputSerial;
use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize};
use casement_protocols::vocab::Toggle;
use smithay::utils::{Logical, Point, Rectangle, Serial, Size};

/// A smithay serial as the model's input serial.
pub fn serial_of(serial: Serial) -> InputSerial {
    InputSerial(u32::from(serial))
}

/// The model's input serial as smithay's.
pub fn serial_to(serial: InputSerial) -> Serial {
    Serial::from(serial.0)
}

/// A smithay logical rectangle as ours (a negative extent becomes empty).
pub fn rect_of(rect: Rectangle<i32, Logical>) -> LogicalRect {
    LogicalRect::new(point_of(rect.loc), size_of(rect.size))
}

/// Our logical rectangle as smithay's.
pub fn rect_to(rect: LogicalRect) -> Rectangle<i32, Logical> {
    Rectangle::new(point_to(rect.origin), size_to(rect.size))
}

/// A smithay logical point as ours.
pub fn point_of(point: Point<i32, Logical>) -> LogicalPoint {
    LogicalPoint {
        x: point.x,
        y: point.y,
    }
}

/// Our logical point as smithay's.
pub fn point_to(point: LogicalPoint) -> Point<i32, Logical> {
    Point::from((point.x, point.y))
}

/// A smithay logical size as ours (negative extents become zero).
pub fn size_of(size: Size<i32, Logical>) -> LogicalSize {
    LogicalSize {
        w: u32::try_from(size.w).unwrap_or(0),
        h: u32::try_from(size.h).unwrap_or(0),
    }
}

/// Our logical size as smithay's (extents beyond `i32::MAX` saturate).
pub fn size_to(size: LogicalSize) -> Size<i32, Logical> {
    Size::from((
        i32::try_from(size.w).unwrap_or(i32::MAX),
        i32::try_from(size.h).unwrap_or(i32::MAX),
    ))
}

/// The only place a smithay `bool` becomes ours.
pub fn toggle_of(on: bool) -> Toggle {
    if on { Toggle::On } else { Toggle::Off }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rectangles_and_serials_round_trip_through_smithay() {
        let ours = LogicalRect::new(
            LogicalPoint { x: -40, y: 25 },
            LogicalSize { w: 800, h: 600 },
        );
        assert_eq!(rect_of(rect_to(ours)), ours);
        assert_eq!(serial_of(serial_to(InputSerial(77))), InputSerial(77));
    }

    #[test]
    fn huge_extents_saturate_and_bools_become_toggles() {
        // smithay refuses negative sizes itself (a debug assertion in `Size::from`).
        assert_eq!(size_of(Size::from((0, 10))), LogicalSize { w: 0, h: 10 });
        assert_eq!(
            size_to(LogicalSize { w: u32::MAX, h: 1 }),
            Size::from((i32::MAX, 1))
        );
        assert_eq!(toggle_of(true), Toggle::On);
        assert_eq!(toggle_of(false), Toggle::Off);
    }
}
