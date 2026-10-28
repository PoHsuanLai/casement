//! `ext_image_copy_capture_cursor_session_v1`: a client watches where the pointer is on an
//! output and, as its own capture source, what it looks like. smithay owns the wire; this
//! module keeps the sessions ([`CursorBook`]) and says where the cursor is on the output's
//! frame ([`place`], pure).

use crate::consent::{self, Held, Mortal, Pushed};
use casement_core::geometry::{
    LogicalPoint, LogicalRect, LogicalSize, PhysicalSize, buffer_size, to_physical, turn_point,
    turn_size,
};
use casement_core::{ClientKey, OutputKey};
use casement_protocols::vocab::Transform;
use ds_core::geometry::scale::Scale;
use smithay::utils::IsAlive;
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
    /// The hotspot's offset from the cursor image's top-left corner, in the pixels of the image
    /// as sent (turned with the output).
    pub hotspot: (i32, i32),
    /// The image as sent: its size at the output's scale, with width and height swapped under a
    /// quarter turn.
    pub image: PhysicalSize,
}

/// The size of the cursor image at `scale`, upright: what the element draws.
pub fn image_size(size: LogicalSize, scale: Scale) -> PhysicalSize {
    buffer_size(size, scale)
}

/// The size of the cursor image a cursor capture session's buffers have: [`image_size`] turned
/// with the output's `transform`.
///
/// ```
/// use casement_core::geometry::{LogicalSize, PhysicalSize};
/// use casement_protocols::vocab::Transform;
/// use casement_wayland::image_capture::sent_image_size;
/// use ds_core::geometry::scale::Scale;
///
/// let size = LogicalSize { w: 24, h: 16 };
/// assert_eq!(sent_image_size(size, Scale(120), Transform::R90), PhysicalSize { w: 16, h: 24 });
/// ```
pub fn sent_image_size(size: LogicalSize, scale: Scale, transform: Transform) -> PhysicalSize {
    turn_size(image_size(size, scale), transform)
}

/// Where a cursor whose hotspot is at `at` (global logical pixels), drawn `size` large with its
/// hotspot `hotspot` from the image's corner (both logical), is on the frame of `view`; none
/// when no pixel of the image is on it (the cursor "enters" the capture when its image meets
/// the frame).
///
/// The image's corner is placed the way the cursor element is: `at - hotspot` in logical
/// pixels, relative to the output, scaled and rounded once, so the capture and the screen agree
/// to the pixel at a fractional scale. The hotspot offset is `round(hotspot x scale)` and the
/// position is the corner plus it. Under a transform the position is turned with the frame, the
/// hotspot is turned inside the image (which is sent turned too), so `position - hotspot` is
/// the turned image's corner.
///
/// ```
/// use casement_core::geometry::{LogicalPoint, LogicalSize, PhysicalSize};
/// use casement_protocols::vocab::Transform;
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
/// // The corner is round(95 x 1.5), round(47 x 1.5) = (143, 71); the hotspot (8, 5).
/// assert_eq!(placed.position, (151, 76));
/// assert_eq!(placed.hotspot, (8, 5));
/// ```
pub fn place(
    view: &CursorView,
    at: LogicalPoint,
    size: LogicalSize,
    hotspot: LogicalPoint,
) -> Option<Placement> {
    let point = |p: LogicalPoint| {
        let rect = to_physical(LogicalRect::new(p, LogicalSize::ZERO), view.scale);
        (rect.x, rect.y)
    };
    let corner = point(LogicalPoint {
        x: at.x.saturating_sub(hotspot.x).saturating_sub(view.origin.x),
        y: at.y.saturating_sub(hotspot.y).saturating_sub(view.origin.y),
    });
    let held = point(hotspot);
    let image = image_size(size, view.scale);
    let upright = turn_size(view.mode, view.transform);
    let meets = |from: i32, length: u32, frame: u32| {
        i64::from(from) < i64::from(frame) && i64::from(from) + i64::from(length) > 0
    };
    if !(meets(corner.0, image.w, upright.w) && meets(corner.1, image.h, upright.h)) {
        return None;
    }
    let side = |n: u32| i32::try_from(n).ok();
    let frame = (side(upright.w)?, side(upright.h)?);
    let inside = (side(image.w)?, side(image.h)?);
    Some(Placement {
        position: turn_point(
            view.transform,
            (
                corner.0.saturating_add(held.0),
                corner.1.saturating_add(held.1),
            ),
            frame,
        ),
        hotspot: turn_point(view.transform, held, inside),
        image: turn_size(image, view.transform),
    })
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

/// A cursor session that waits for the person's answer about the client's screen: nothing is
/// told to it, not even where the pointer is, until they allow it (NEW-CAS-4).
#[derive(Debug)]
#[non_exhaustive]
pub struct ParkedCursor {
    /// The session.
    pub session: CursorSession,
    /// The output it watches.
    pub output: OutputKey,
}

impl Mortal for ParkedCursor {
    fn alive(&self) -> bool {
        self.session.alive()
    }
}

/// The cursor sessions held, and the frame requests that wait for the image to change.
#[derive(Debug, Default)]
pub struct CursorBook {
    held: BTreeMap<CursorSessionId, CursorHold>,
    waiting: BTreeMap<CursorSessionId, Frame>,
    parked: Held<ParkedCursor>,
    next: u64,
}

impl CursorBook {
    /// Keeps `session` for `client` until the answer about its screen comes out; a client with
    /// too many waiting has it dropped (the client hears `stopped`).
    pub fn park(&mut self, client: ClientKey, session: CursorSession, output: OutputKey) {
        if let Pushed::Full(refused) = self.parked.push(client, ParkedCursor { session, output }) {
            drop(refused);
        }
    }

    /// The sessions `client` has waiting, oldest first.
    pub fn take_parked(&mut self, client: ClientKey) -> Vec<ParkedCursor> {
        self.parked.take(client)
    }

    /// Drops every session of `client`, held ones included: each tells its client `stopped`
    /// (the power was refused or taken back).
    pub fn stop_client(&mut self, client: ClientKey) {
        let ended: Vec<CursorSessionId> = self
            .held
            .iter()
            .filter(|(_, hold)| {
                hold.session.client().as_ref().and_then(consent::key_of) == Some(client)
            })
            .map(|(id, _)| *id)
            .collect();
        for id in ended {
            self.release(id);
        }
    }

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
            // 1.5x: the corner (95, 47) is 142.5 and 70.5, which round up to (143, 71); the
            // hotspot (5, 3) is 7.5 and 4.5, which round to (8, 5); the position is their sum.
            (180, at(100, 50), Some(((151, 76), (8, 5)))),
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
    fn the_corner_lands_where_the_cursor_element_puts_it() {
        // The element rounds `at - hotspot - origin` times the scale once; the capture's
        // `position - hotspot` is that corner at every fractional scale and sub-pixel phase.
        for scale in [130_u32, 150, 180, 210, 270] {
            for origin in [0, 1280, 1281] {
                for x in 1280..1300 {
                    let view = view(origin, scale, (1920, 1080), Transform::Normal);
                    let Some(p) = place(&view, at(x, 77), SIZE, HOT) else {
                        continue;
                    };
                    let expected = to_physical(
                        LogicalRect::new(at(x - HOT.x - origin, 77 - HOT.y), LogicalSize::ZERO),
                        Scale(scale),
                    );
                    assert_eq!(
                        (p.position.0 - p.hotspot.0, p.position.1 - p.hotspot.1),
                        (expected.x, expected.y),
                        "x {x} origin {origin} scale {scale}"
                    );
                }
            }
        }
    }

    #[test]
    fn a_second_output_is_measured_from_its_own_corner() {
        let second = view(1920, 120, (1920, 1080), Transform::Normal);
        assert_eq!(placed(&second, at(2020, 50)), Some(((100, 50), (5, 3))));
        assert_eq!(placed(&second, at(100, 50)), None, "that is on the first");
    }

    #[test]
    fn a_transform_turns_the_position_and_the_hotspot_with_the_frame() {
        // The logical output is 1920 x 1080; a quarter turn makes the frame 1080 x 1920. The
        // cursor image is 24 x 24 with its hotspot 5 right and 3 down, turned with the frame.
        let table = [
            (Transform::Normal, (1920, 1080), (100, 50), (5, 3)),
            (Transform::R90, (1080, 1920), (1030, 100), (21, 5)),
            (Transform::R180, (1920, 1080), (1820, 1030), (19, 21)),
            (Transform::R270, (1080, 1920), (50, 1820), (3, 19)),
            (Transform::Flipped, (1920, 1080), (1820, 50), (19, 3)),
            (Transform::Flipped90, (1080, 1920), (50, 100), (3, 5)),
            (Transform::Flipped180, (1920, 1080), (100, 1030), (5, 21)),
            (Transform::Flipped270, (1080, 1920), (1030, 1820), (21, 19)),
        ];
        for (transform, mode, position, hotspot) in table {
            let view = view(0, 120, mode, transform);
            assert_eq!(
                placed(&view, at(100, 50)),
                Some((position, hotspot)),
                "{transform:?}"
            );
        }
    }

    #[test]
    fn the_turned_image_sits_where_the_turned_cursor_does() {
        // `position - hotspot` is the turned image's top-left corner: the box the upright image
        // fills, turned with the frame, starts there.
        let mode = (1080, 1920);
        for transform in [Transform::R90, Transform::R270, Transform::Flipped90] {
            let view = view(0, 120, mode, transform);
            let p = place(&view, at(100, 50), SIZE, HOT).expect("on the frame");
            let corner = (p.position.0 - p.hotspot.0, p.position.1 - p.hotspot.1);
            let (w, h) = (1920, 1080);
            let upright_box = [(95, 47), (95 + 24, 47 + 24)];
            let turned: Vec<_> = upright_box
                .iter()
                .map(|&point| turn_point(transform, point, (w, h)))
                .collect();
            let low = (turned[0].0.min(turned[1].0), turned[0].1.min(turned[1].1));
            assert_eq!(corner, low, "{transform:?}");
            assert_eq!(p.image, PhysicalSize { w: 24, h: 24 });
        }
    }

    #[test]
    fn a_scaled_and_turned_output_composes_both() {
        // 3840 x 2160 at 2x is 1920 x 1080 logical; turned 90 degrees the frame is 2160 x 3840.
        // The 48 x 48 image has its hotspot (10, 6) turned to (48 - 6, 10).
        let view = view(0, 240, (2160, 3840), Transform::R90);
        assert_eq!(
            placed(&view, at(100, 50)),
            Some(((2160 - 100, 200), (42, 10)))
        );
    }

    #[test]
    fn a_wide_image_is_sent_turned_with_the_output() {
        let wide = LogicalSize { w: 24, h: 16 };
        let table = [
            (Transform::Normal, 120, (24, 16)),
            (Transform::R90, 120, (16, 24)),
            (Transform::Flipped270, 180, (24, 36)),
            (Transform::R180, 180, (36, 24)),
        ];
        for (transform, scale, (w, h)) in table {
            assert_eq!(
                sent_image_size(wide, Scale(scale), transform),
                PhysicalSize { w, h },
                "{transform:?} at {scale}"
            );
        }
    }

    #[test]
    fn the_image_is_as_big_as_its_logical_size_at_the_scale() {
        assert_eq!(image_size(SIZE, Scale(120)), PhysicalSize { w: 24, h: 24 });
        assert_eq!(image_size(SIZE, Scale(180)), PhysicalSize { w: 36, h: 36 });
        assert_eq!(image_size(SIZE, Scale(240)), PhysicalSize { w: 48, h: 48 });
    }
}
