//! `ext_image_copy_capture_cursor_session_v1`: a client watches where the pointer is on an
//! output and, as its own capture source, what it looks like. smithay owns the wire; this
//! module keeps the sessions ([`CursorBook`]) and says where the cursor is on the output's
//! frame ([`place`], pure).

use casement_core::OutputKey;
use casement_core::geometry::{
    LogicalPoint, LogicalRect, LogicalSize, PhysicalSize, Transform, buffer_size, to_physical,
};
use ds_core::geometry::scale::Scale;
use smithay::wayland::image_copy_capture::{CursorSession, Frame};
use std::collections::BTreeMap;

/// One cursor capture session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CursorSessionId(pub u64);

/// An output as a cursor sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct CursorView {
    /// The output's top-left corner in global logical coordinates.
    pub origin: LogicalPoint,
    /// Its scale.
    pub scale: Scale,
    /// Its mode: the size of a frame.
    pub mode: PhysicalSize,
    /// Its transform.
    pub transform: Transform,
}

impl CursorView {
    /// A view of an output.
    pub fn new(
        origin: LogicalPoint,
        scale: Scale,
        mode: PhysicalSize,
        transform: Transform,
    ) -> CursorView {
        CursorView {
            origin,
            scale,
            mode,
            transform,
        }
    }
}

/// Where the cursor is on a frame, in the frame's pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Placement {
    /// The hotspot, from the frame's top-left corner; negative or past the frame when only
    /// part of the cursor is on it.
    pub position: (i32, i32),
    /// The hotspot's offset from the cursor image's top-left corner, in the image's pixels.
    pub hotspot: (i32, i32),
}

/// The size of the cursor image at `scale`: what a cursor capture session's buffers are.
pub fn image_size(size: LogicalSize, scale: Scale) -> PhysicalSize {
    buffer_size(size, scale)
}

/// Where a cursor whose hotspot is at `at` (global logical pixels), drawn `size` large with its
/// hotspot `hotspot` from the image's corner (both logical), is on the frame of `view`; none
/// when no pixel of the image is on it (the cursor "enters" the capture when its image meets
/// the frame). The position is in the frame's pixels, so under a transform the hotspot is
/// turned with it; the image itself is sent upright.
///
/// ```
/// use casement_core::geometry::{LogicalPoint, LogicalSize, PhysicalSize, Transform};
/// use casement_wayland::image_capture::{CursorView, place};
/// use ds_core::geometry::scale::Scale;
///
/// let view = CursorView::new(
///     LogicalPoint { x: 0, y: 0 },
///     Scale(180),
///     PhysicalSize { w: 1920, h: 1080 },
///     Transform::Normal,
/// );
/// let size = LogicalSize { w: 24, h: 24 };
/// let at = LogicalPoint { x: 100, y: 50 };
/// let placed = place(&view, at, size, LogicalPoint { x: 5, y: 3 }).expect("on the frame");
/// assert_eq!(placed.position, (150, 75));
/// ```
pub fn place(
    view: &CursorView,
    at: LogicalPoint,
    size: LogicalSize,
    hotspot: LogicalPoint,
) -> Option<Placement> {
    let local = LogicalPoint {
        x: at.x.saturating_sub(view.origin.x),
        y: at.y.saturating_sub(view.origin.y),
    };
    let point = |p: LogicalPoint| {
        let rect = to_physical(LogicalRect::new(p, LogicalSize::ZERO), view.scale);
        (rect.x, rect.y)
    };
    let (x, y) = point(local);
    let held = point(hotspot);
    let image = image_size(size, view.scale);
    let upright = turned(view.mode, view.transform);
    let meets = |from: i64, length: u32, frame: u32| {
        from < i64::from(frame) && from + i64::from(length) > 0
    };
    let corner = (
        i64::from(x) - i64::from(held.0),
        i64::from(y) - i64::from(held.1),
    );
    if !(meets(corner.0, image.w, upright.w) && meets(corner.1, image.h, upright.h)) {
        return None;
    }
    let area = (
        i32::try_from(upright.w).ok()?,
        i32::try_from(upright.h).ok()?,
    );
    Some(Placement {
        position: turn(view.transform, (x, y), area),
        hotspot: held,
    })
}

/// The frame's size before the transform: width and height swap for the quarter turns.
fn turned(mode: PhysicalSize, transform: Transform) -> PhysicalSize {
    match transform {
        Transform::R90 | Transform::R270 | Transform::Flipped90 | Transform::Flipped270 => {
            PhysicalSize {
                w: mode.h,
                h: mode.w,
            }
        }
        _ => mode,
    }
}

/// The point `(x, y)` of an upright area of `area` as the transformed frame has it.
fn turn(transform: Transform, (x, y): (i32, i32), (w, h): (i32, i32)) -> (i32, i32) {
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

/// A cursor session held: its output and the last image it was sent.
#[derive(Debug)]
#[non_exhaustive]
pub struct CursorHold {
    /// The session; dropping it tells the client `stopped`.
    pub session: CursorSession,
    /// The output it watches.
    pub output: OutputKey,
    /// The pixels of the last cursor image sent (a new frame is held back until the image
    /// differs).
    pub shown: Option<Vec<u8>>,
}

/// The cursor sessions held, and the frame requests that wait for the image to change.
#[derive(Debug, Default)]
pub struct CursorBook {
    held: BTreeMap<CursorSessionId, CursorHold>,
    waiting: BTreeMap<CursorSessionId, Frame>,
    next: u64,
}

impl CursorBook {
    /// Keeps `session`, which watches `output`.
    pub fn hold(&mut self, session: CursorSession, output: OutputKey) -> CursorSessionId {
        let id = CursorSessionId(self.next);
        self.next = self.next.saturating_add(1);
        self.held.insert(
            id,
            CursorHold {
                session,
                output,
                shown: None,
            },
        );
        id
    }

    /// The session `id`.
    pub fn get(&self, id: CursorSessionId) -> Option<&CursorHold> {
        self.held.get(&id)
    }

    /// The session `id`, mutably.
    pub fn get_mut(&mut self, id: CursorSessionId) -> Option<&mut CursorHold> {
        self.held.get_mut(&id)
    }

    /// The sessions watching `output`.
    pub fn on(&self, output: &OutputKey) -> Vec<CursorSessionId> {
        self.held
            .iter()
            .filter(|(_, hold)| hold.output == *output)
            .map(|(id, _)| *id)
            .collect()
    }

    /// Keeps `frame`, a request of `id` for an image that has not changed (one it replaces is
    /// dropped, which fails it).
    pub fn wait(&mut self, id: CursorSessionId, frame: Frame) {
        self.waiting.insert(id, frame);
    }

    /// The request of `id` that waits, to serve it again.
    pub fn take_waiting(&mut self, id: CursorSessionId) -> Option<Frame> {
        self.waiting.remove(&id)
    }

    /// The client destroyed session `id`.
    pub fn release(&mut self, id: CursorSessionId) {
        self.waiting.remove(&id);
        self.held.remove(&id);
    }

    /// `output` was unplugged: its sessions are dropped (their clients hear `stopped`).
    pub fn forget_output(&mut self, output: &OutputKey) {
        for id in self.on(output) {
            self.release(id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIZE: LogicalSize = LogicalSize { w: 24, h: 24 };
    const HOT: LogicalPoint = LogicalPoint { x: 5, y: 3 };

    fn view(origin: i32, scale: u32, mode: (u32, u32), transform: Transform) -> CursorView {
        CursorView::new(
            LogicalPoint { x: origin, y: 0 },
            Scale(scale),
            PhysicalSize {
                w: mode.0,
                h: mode.1,
            },
            transform,
        )
    }

    fn at(x: i32, y: i32) -> LogicalPoint {
        LogicalPoint { x, y }
    }

    /// Position and hotspot of the cursor, or none.
    fn placed(view: &CursorView, point: LogicalPoint) -> Option<((i32, i32), (i32, i32))> {
        place(view, point, SIZE, HOT).map(|p| (p.position, p.hotspot))
    }

    #[test]
    fn the_scale_moves_the_hotspot_and_the_image_to_physical_pixels() {
        let full = (1920, 1080);
        let table = [
            // 1x: nothing moves.
            (120, at(100, 50), Some(((100, 50), (5, 3)))),
            // 1.5x: 7.5 and 4.5 round away from zero.
            (180, at(100, 50), Some(((150, 75), (8, 5)))),
            (240, at(100, 50), Some(((200, 100), (10, 6)))),
            // Partly on the frame still counts: 21 of the 24 columns show at x = 2.
            (120, at(2, 2), Some(((2, 2), (5, 3)))),
            (120, at(-10, 10), Some(((-10, 10), (5, 3)))),
            // Nothing of the image on the frame.
            (120, at(-100, -100), None),
            (120, at(1930, 10), None),
            (120, at(10, 1100), None),
        ];
        for (scale, point, expected) in table {
            let view = view(0, scale, full, Transform::Normal);
            assert_eq!(placed(&view, point), expected, "{point:?} at {scale}");
        }
    }

    #[test]
    fn a_second_output_is_measured_from_its_own_corner() {
        let second = view(1920, 120, (1920, 1080), Transform::Normal);
        assert_eq!(placed(&second, at(2020, 50)), Some(((100, 50), (5, 3))));
        assert_eq!(placed(&second, at(100, 50)), None, "that is on the first");
    }

    #[test]
    fn a_transform_turns_the_position_with_the_frame() {
        // The logical output is 1920 x 1080; a quarter turn makes the frame 1080 x 1920.
        let table = [
            (Transform::Normal, (1920, 1080), (100, 50)),
            (Transform::R90, (1080, 1920), (1030, 100)),
            (Transform::R180, (1920, 1080), (1820, 1030)),
            (Transform::R270, (1080, 1920), (50, 1820)),
            (Transform::Flipped, (1920, 1080), (1820, 50)),
            (Transform::Flipped90, (1080, 1920), (50, 100)),
            (Transform::Flipped180, (1920, 1080), (100, 1030)),
            (Transform::Flipped270, (1080, 1920), (1030, 1820)),
        ];
        for (transform, mode, position) in table {
            let view = view(0, 120, mode, transform);
            assert_eq!(
                placed(&view, at(100, 50)),
                Some((position, (5, 3))),
                "{transform:?}"
            );
        }
    }

    #[test]
    fn a_scaled_and_turned_output_composes_both() {
        // 3840 x 2160 at 2x is 1920 x 1080 logical; turned 90 degrees the frame is 2160 x 3840.
        let view = view(0, 240, (2160, 3840), Transform::R90);
        assert_eq!(
            placed(&view, at(100, 50)),
            Some(((2160 - 100, 200), (10, 6)))
        );
    }

    #[test]
    fn the_image_is_as_big_as_its_logical_size_at_the_scale() {
        assert_eq!(image_size(SIZE, Scale(120)), PhysicalSize { w: 24, h: 24 });
        assert_eq!(image_size(SIZE, Scale(180)), PhysicalSize { w: 36, h: 36 });
        assert_eq!(image_size(SIZE, Scale(240)), PhysicalSize { w: 48, h: 48 });
    }
}
