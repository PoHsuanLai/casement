//! What wlcs asks of a compositor, as one closed enum (SP5: client, window position, pointer
//! absolute and relative motion and buttons, touch down, move and up is the whole surface), and
//! the adapter that applies it to the fixture. Coordinates arrive as `wl_fixed` and are
//! converted at the edge ([`ffi`](crate)) into sub-pixels.

use casement_core::geometry::{LogicalPoint, SubPoint};
use casement_core::input::{ButtonCode, Edge};
use std::os::unix::net::UnixStream;

/// A wlcs client's connection, named by the fd number wlcs knows it by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WlcsClient(pub i32);

/// A surface of a wlcs client, by its protocol object id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WlcsSurface(pub u32);

/// One wlcs touch point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WlcsTouchId(pub u32);

/// One request from wlcs' thread to the compositor thread.
#[derive(Debug)]
pub enum WlcsEvent {
    /// Stop the compositor.
    Exit,
    /// A new client on the server end of this socket pair.
    NewClient {
        /// The server end.
        stream: UnixStream,
        /// How wlcs names the client.
        client: WlcsClient,
    },
    /// Place a client's toplevel at a global logical point.
    PositionWindow {
        /// The client.
        client: WlcsClient,
        /// Its surface.
        surface: WlcsSurface,
        /// Where its top-left goes.
        at: LogicalPoint,
    },
    /// Move the pointer to a global point.
    PointerTo(SubPoint),
    /// Move the pointer by a delta.
    PointerBy(SubPoint),
    /// A pointer button went down or up.
    Button {
        /// The evdev button.
        button: ButtonCode,
        /// Down or up.
        edge: Edge,
    },
    /// A finger touched down.
    TouchDown {
        /// The touch point.
        id: WlcsTouchId,
        /// Where.
        at: SubPoint,
    },
    /// A finger moved.
    TouchMove {
        /// The touch point.
        id: WlcsTouchId,
        /// Where to.
        at: SubPoint,
    },
    /// A finger lifted.
    TouchUp {
        /// The touch point.
        id: WlcsTouchId,
    },
}

/// Applies one event to the fixture.
pub fn apply(fixture: &mut casement_testkit::Fixture, event: WlcsEvent) {
    let _ = (fixture, event);
    todo!("P0-T-wlcs: each WlcsEvent through Fixture::connect, FixtureInput and the model")
}
