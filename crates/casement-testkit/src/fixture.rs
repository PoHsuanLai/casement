//! The fixture: the real `State` on the headless backend, a Wayland display, test clients on
//! socket pairs, pumped on one thread until nothing moves.
//!
//! Mechanics frozen from SP3: each client's end of its socket pair is non-blocking; the
//! display's fd is a duplicate of its poll fd inside a calloop source (no `unsafe`); quiescence
//! is a round in which no server message was dispatched, no timer fired, no client event was
//! dispatched and no request was newly queued; `pump` stamps the virtual now into every
//! transcript; `advance` pumps at the current instant before the first stop, then stops at
//! every due instant, so chopping an advance differently gives the same transcript.

use crate::fakes::{FakePalmrest, FakeSession};
use crate::input::FixtureInput;
use crate::spec::FixtureSpec;
use casement::State;
use casement::display::SharedDisplay;
use casement_core::geometry::LogicalRect;
use casement_core::policy::ConnectionRole;
use casement_core::time::TimerKey;
use casement_core::{Model, OutputName, WindowId};
use casement_testclient::{ClientError, Pumped, TestClient};
use smithay::reexports::calloop::EventLoop;
use std::collections::BTreeMap;
use std::time::Duration;

/// One running compositor and its clients.
pub struct Fixture {
    #[expect(dead_code, reason = "P0-B-headless: the compositor")]
    state: State,
    #[expect(dead_code, reason = "P0-B-headless: the display")]
    display: SharedDisplay,
    #[expect(dead_code, reason = "P0-B-headless: the event loop")]
    event_loop: EventLoop<'static, State>,
    #[expect(dead_code, reason = "P0-B-headless: the test clients")]
    clients: BTreeMap<ClientId, TestClient>,
}

impl std::fmt::Debug for Fixture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Fixture")
    }
}

/// A test client's number in its fixture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ClientId(pub u32);

/// A number of frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FrameCount(pub u32);

/// What an advance did.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Advanced {
    /// Frames presented, over every output.
    pub frames: u32,
    /// Frame callbacks sent.
    pub callbacks: u32,
    /// Timers fired, in order.
    pub timers: Vec<TimerKey>,
}

/// What a shot captures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShotTarget {
    /// A whole output's last frame.
    Output(OutputName),
    /// One window's surface tree, rendered alone.
    Window(WindowId),
    /// A region of an output's last frame (global logical).
    Region(OutputName, LogicalRect),
}

/// The element tree of the last frames: kind, layer, rectangles, opacity, origin filter.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneDump(pub serde_json::Value);

/// Why a fixture could not start or a client could not join.
#[derive(Debug, thiserror::Error)]
pub enum FixtureError {
    /// The compositor did not start.
    #[error("start: {0}")]
    Start(#[from] casement::StartError),
    /// A client could not connect.
    #[error("client: {0}")]
    Client(#[from] ClientError),
    /// The event loop failed.
    #[error("event loop: {0}")]
    Loop(String),
}

impl Fixture {
    /// casement as `spec` describes it, pumped until it is quiet: outputs plugged, no client.
    pub fn start(spec: FixtureSpec) -> Result<Fixture, FixtureError> {
        let _ = spec;
        todo!("P0-B-headless: Fixture::start")
    }

    /// A new client arriving through a socket of `role`.
    pub fn connect(&mut self, role: ConnectionRole) -> ClientId {
        let _ = role;
        todo!("P0-B-headless: Fixture::connect")
    }

    /// Client `id`.
    pub fn client(&mut self, id: ClientId) -> &mut TestClient {
        let _ = id;
        todo!("P0-B-headless: Fixture::client")
    }

    /// Dispatches the server and every client at the current instant until nothing moves.
    pub fn pump(&mut self) -> Pumped {
        todo!("P0-B-headless: Fixture::pump")
    }

    /// Moves the clock `by`, stopping at every timer and vblank on the way and pumping at each.
    pub fn advance(&mut self, by: Duration) -> Advanced {
        let _ = by;
        todo!("P0-B-headless: Fixture::advance")
    }

    /// Advances until `output` has presented `n` more frames.
    pub fn frames(&mut self, output: &OutputName, n: FrameCount) -> Advanced {
        let _ = (output, n);
        todo!("P0-B-headless: Fixture::frames")
    }

    /// Advances until `predicate` holds on the model, at most `bound` of virtual time.
    pub fn run_until(&mut self, predicate: impl Fn(&Model) -> bool, bound: Duration) -> Advanced {
        let _ = (predicate, bound);
        todo!("P0-B-headless: Fixture::run_until")
    }

    /// The virtual input devices.
    pub fn input(&mut self) -> FixtureInput<'_> {
        FixtureInput::of(self)
    }

    /// The fake palmrest feed (bodies at P3).
    pub fn palmrest(&mut self) -> &mut FakePalmrest {
        todo!("P3-Z: Fixture::palmrest")
    }

    /// The fake session (bodies at P1).
    pub fn session(&mut self) -> &mut FakeSession {
        todo!("P1-D1: Fixture::session")
    }

    /// A shot of `target`, from the last presented frame.
    pub fn shot(&mut self, target: ShotTarget) -> image::RgbaImage {
        let _ = target;
        todo!("P0-B-headless: Fixture::shot")
    }

    /// The last frames' element tree.
    pub fn scene(&self) -> SceneDump {
        todo!("P0-B-headless: Fixture::scene")
    }

    /// The model.
    pub fn model(&self) -> &Model {
        todo!("P0-B-headless: Fixture::model")
    }
}
