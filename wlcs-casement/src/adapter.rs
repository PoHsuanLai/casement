//! What wlcs asks of a compositor, as one closed enum (SP5: a client, a window position, pointer
//! absolute and relative motion and buttons, touch down, move and up is the whole surface), and
//! the [`Compositor`] seam that applies it. Coordinates stay `wl_fixed` ([`Fixed`], 1/256 of a
//! pixel) all the way here, because wlcs' tests use fractions a 1/120 sub-pixel would round; the
//! implementation of the seam converts at its own edge.

use casement_protocols::vocab::PressEdge;
use std::os::fd::OwnedFd;
use std::sync::mpsc::Sender;

/// A `wl_fixed_t`: signed 24.8 fixed point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Fixed(pub i32);

impl Fixed {
    /// The value as a float (`wl_fixed_to_double`).
    pub fn to_f64(self) -> f64 {
        f64::from(self.0) / 256.0
    }
}

/// A point or a delta in [`Fixed`] units.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixedPoint {
    /// Rightward.
    pub x: Fixed,
    /// Downward.
    pub y: Fixed,
}

impl FixedPoint {
    /// The point as floats.
    pub fn to_f64(self) -> (f64, f64) {
        (self.x.to_f64(), self.y.to_f64())
    }
}

/// A whole-pixel position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    /// Rightward.
    pub x: i32,
    /// Downward.
    pub y: i32,
}

/// A wlcs client's connection, named by the fd number wlcs knows it by (the number of the
/// client end of its socket pair, which `wl_display_get_fd` reports back).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WlcsClient(pub i32);

/// A surface of a wlcs client, by its protocol object id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WlcsSurface(pub u32);

/// One wlcs touch point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WlcsTouchId(pub u32);

/// An evdev button code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Button(pub u32);

/// One request from wlcs' thread to the compositor thread.
#[derive(Debug)]
pub enum WlcsEvent {
    /// Stop the compositor.
    Exit,
    /// A new client: the compositor adopts the server end of a fresh socket pair and sends the
    /// client end, and the name wlcs will know it by, back.
    NewClient {
        /// Where the client end goes.
        reply: Sender<(WlcsClient, OwnedFd)>,
    },
    /// Place a client's toplevel at a global point.
    PositionWindow {
        /// The client.
        client: WlcsClient,
        /// Its surface.
        surface: WlcsSurface,
        /// Where its top-left goes.
        at: Position,
    },
    /// Move the pointer to a global point.
    PointerTo(FixedPoint),
    /// Move the pointer by a delta.
    PointerBy(FixedPoint),
    /// A pointer button changed state.
    Button {
        /// The evdev button.
        button: Button,
        /// Down or up.
        edge: PressEdge,
    },
    /// A finger touched down.
    TouchDown {
        /// The touch point.
        id: WlcsTouchId,
        /// Where.
        at: FixedPoint,
    },
    /// A finger moved.
    TouchMove {
        /// The touch point.
        id: WlcsTouchId,
        /// Where to.
        at: FixedPoint,
    },
    /// A finger lifted.
    TouchUp {
        /// The touch point.
        id: WlcsTouchId,
    },
}

/// Whether the compositor thread carries on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flow {
    /// Keep serving.
    Continue,
    /// [`WlcsEvent::Exit`] arrived.
    Stop,
}

/// The seam between wlcs and a compositor: one call per [`WlcsEvent`]. The implementation is
/// [`host::Host`](crate::host::Host), casement's own `Fixture` (the real `State` on the headless
/// backend).
pub trait Compositor {
    /// Applies one request.
    fn apply(&mut self, event: WlcsEvent) -> Flow;
}

impl WlcsEvent {
    /// This event applied to `compositor`.
    pub fn apply_to(self, compositor: &mut impl Compositor) -> Flow {
        compositor.apply(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_is_24_8() {
        let rows = [
            (0, 0.0),
            (256, 1.0),
            (-256, -1.0),
            (128, 0.5),
            (1, 1.0 / 256.0),
            (-640, -2.5),
        ];
        for (raw, want) in rows {
            assert_eq!(Fixed(raw).to_f64(), want, "raw {raw}");
        }
    }

    #[test]
    fn point_converts_both_axes() {
        let at = FixedPoint {
            x: Fixed(384),
            y: Fixed(-128),
        };
        assert_eq!(at.to_f64(), (1.5, -0.5));
    }

    struct Recorder(Vec<&'static str>);
    impl Compositor for Recorder {
        fn apply(&mut self, event: WlcsEvent) -> Flow {
            match event {
                WlcsEvent::Exit => Flow::Stop,
                WlcsEvent::PointerTo(_) => {
                    self.0.push("to");
                    Flow::Continue
                }
                _ => Flow::Continue,
            }
        }
    }

    #[test]
    fn exit_stops_and_others_continue() {
        let mut rec = Recorder(Vec::new());
        let at = FixedPoint {
            x: Fixed(0),
            y: Fixed(0),
        };
        assert_eq!(WlcsEvent::PointerTo(at).apply_to(&mut rec), Flow::Continue);
        assert_eq!(WlcsEvent::Exit.apply_to(&mut rec), Flow::Stop);
        assert_eq!(rec.0, ["to"]);
    }
}
