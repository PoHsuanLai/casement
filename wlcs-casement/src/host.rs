//! The [`Compositor`] behind the seam: casement's own test-kit [`Fixture`], the real `State` on
//! the headless backend (pixman, virtual clock), driven from one thread.
//!
//! wlcs' clients live in another thread and talk at their own pace, so the loop alternates
//! between taking requests, waiting a moment for a client to speak, and moving the virtual
//! clock forward by the real time that passed: frame callbacks, vblanks and timers then follow
//! wall time as they would on a screen.

use crate::adapter::{
    Button, Compositor, Edge, Fixed, FixedPoint, Flow, Position, WlcsClient, WlcsEvent,
    WlcsSurface, WlcsTouchId,
};
use casement::relax::AckPolicy;
use casement_core::geometry::{LogicalPoint, SubPoint, SubPx};
use casement_core::input::{ButtonCode, DeviceClass, Edge as InputEdge, TouchStep};
use casement_core::policy::ConnectionRole;
use casement_core::window::WindowState;
use casement_core::{DeviceId, WindowId};
use casement_testkit::{ClientId, Fixture, FixtureSpec, TestOrigin};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::time::{Duration, Instant};

/// How long one turn waits for a client to speak.
const WAIT: Duration = Duration::from_millis(1);

/// How much virtual time a placement is given to finish its motion.
const SETTLE: Duration = Duration::from_secs(1);

/// One request and the way to tell wlcs' thread it was carried out.
#[derive(Debug)]
pub struct Request {
    /// What wlcs asked.
    pub event: WlcsEvent,
    /// Told once the compositor has applied the request and settled.
    pub done: Sender<()>,
}

/// The fixture and the devices wlcs' pointer and touch are.
#[derive(Debug)]
pub struct Host {
    fixture: Fixture,
    clients: HashMap<WlcsClient, ClientId>,
    pointer: DeviceId,
    touch: DeviceId,
    /// The windows that have already been activated.
    activated: BTreeSet<WindowId>,
    /// Where wlcs put each toplevel by hand: a toplevel that unmaps and maps again (a null buffer,
    /// then a buffer) is a new window of the same id, and wlcs takes it to be where it left it.
    placed: BTreeMap<(WlcsClient, WlcsSurface), LogicalPoint>,
}

impl Host {
    /// A fixture with a pointer and a touchscreen plugged.
    pub fn start() -> Host {
        let spec = FixtureSpec::desktop().with_ack_policy(AckPolicy::Lenient);
        let mut fixture = Fixture::start(spec).expect("the fixture starts");
        let pointer = fixture.input().plug(
            DeviceClass::Pointer,
            TestOrigin::Physical(DeviceClass::Pointer),
        );
        let touch = fixture
            .input()
            .plug(DeviceClass::Touch, TestOrigin::Physical(DeviceClass::Touch));
        Host {
            fixture,
            clients: HashMap::new(),
            pointer,
            touch,
            activated: BTreeSet::new(),
            placed: BTreeMap::new(),
        }
    }

    /// Serves `requests` until [`WlcsEvent::Exit`] or until wlcs' side hangs up.
    pub fn serve(mut self, requests: Receiver<Request>) {
        let mut last = Instant::now();
        loop {
            loop {
                match requests.try_recv() {
                    Ok(request) => {
                        let flow = self.apply(request.event);
                        let _ = request.done.send(());
                        if flow == Flow::Stop {
                            return;
                        }
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => return,
                }
            }
            self.fixture.wait(WAIT);
            let now = Instant::now();
            self.fixture.advance(now.duration_since(last));
            self.activate_new();
            last = now;
        }
    }

    /// Activates each window that has mapped since the last turn. wlcs takes a newly mapped
    /// window to be the active one; casement does not key a window nobody asked for (A7), so
    /// the shim asks as the person would (a valid token).
    fn activate_new(&mut self) {
        let fresh: Vec<WindowId> = self
            .fixture
            .model()
            .windows()
            .keys()
            .filter(|window| !self.activated.contains(window) || self.remapped(**window))
            .copied()
            .collect();
        for window in fresh {
            self.fixture.activate_window(window);
            self.activated.insert(window);
            self.replace_by_hand(window);
        }
    }

    /// Whether `window` mapped again after wlcs put it somewhere: a toplevel that unmaps (a null
    /// buffer) keeps its id and maps as a fresh window, one casement has not placed yet, while
    /// wlcs takes it to be where it left it.
    fn remapped(&self, window: WindowId) -> bool {
        let fresh = matches!(
            self.fixture
                .model()
                .windows()
                .get(&window)
                .map(|held| &held.state),
            Some(WindowState::New(_))
        );
        fresh && self.spot_of(window).is_some()
    }

    fn spot_of(&self, window: WindowId) -> Option<LogicalPoint> {
        self.placed
            .iter()
            .find(|((client, surface), _)| self.client_surface(*client, *surface) == Some(window))
            .map(|(_, at)| *at)
    }

    /// A window that mapped again goes back where wlcs had put the surface it is.
    fn replace_by_hand(&mut self, window: WindowId) {
        if let Some(at) = self.spot_of(window) {
            self.fixture.place_window_by_hand(window, at);
            self.fixture.advance(SETTLE);
            self.fixture.refocus_pointer();
        }
    }

    fn client_surface(&self, client: WlcsClient, surface: WlcsSurface) -> Option<WindowId> {
        let id = *self.clients.get(&client)?;
        self.fixture.toplevel_of(id, surface.0)
    }
}

impl Compositor for Host {
    fn apply(&mut self, event: WlcsEvent) -> Flow {
        match event {
            WlcsEvent::Exit => return Flow::Stop,
            WlcsEvent::NewClient { reply } => {
                let (id, fd) = self.fixture.adopt_socket(ConnectionRole::Public);
                let client = WlcsClient(std::os::fd::AsRawFd::as_raw_fd(&fd));
                self.clients.insert(client, id);
                let _ = reply.send((client, fd));
            }
            WlcsEvent::PositionWindow {
                client,
                surface,
                at,
            } => {
                let layer = self
                    .clients
                    .get(&client)
                    .and_then(|id| self.fixture.layer_of(*id, surface.0));
                if let Some(layer) = layer {
                    // A layer surface sits where its anchors put it; wlcs' layer tests still
                    // move surfaces about, so the shim pins them.
                    self.fixture.place_layer(layer, whole(at));
                    self.fixture.advance(SETTLE);
                    self.fixture.refocus_pointer();
                } else if let Some(window) = self.client_surface(client, surface) {
                    self.placed.insert((client, surface), whole(at));
                    self.fixture.place_window_by_hand(window, whole(at));
                    // The window glides to its place; wlcs' next step hit-tests where it is.
                    self.fixture.advance(SETTLE);
                    self.fixture.refocus_pointer();
                }
            }
            WlcsEvent::PointerTo(at) => {
                let device = self.pointer;
                self.fixture.input().pointer_to(device, floor(at));
            }
            WlcsEvent::PointerBy(by) => {
                let device = self.pointer;
                self.fixture.input().pointer_by(device, sub(by));
            }
            WlcsEvent::Button { button, edge } => {
                let device = self.pointer;
                self.fixture
                    .input()
                    .button(device, code(button), input_edge(edge));
                if input_edge(edge) == InputEdge::Release {
                    // A dragged window glides after the pointer (a spring); the test's next step
                    // hit-tests where it dropped.
                    self.fixture.advance(SETTLE);
                    self.fixture.refocus_pointer();
                }
            }
            WlcsEvent::TouchDown { id, at } => self.touch_step(TouchStep::Down {
                slot: slot(id),
                at: floor(at),
            }),
            WlcsEvent::TouchMove { id, at } => self.touch_step(TouchStep::Motion {
                slot: slot(id),
                at: floor(at),
            }),
            WlcsEvent::TouchUp { id } => self.touch_step(TouchStep::Up { slot: slot(id) }),
        }
        Flow::Continue
    }
}

impl Host {
    /// A touch step, closed by its frame.
    fn touch_step(&mut self, step: TouchStep) {
        let device = self.touch;
        let mut input = self.fixture.input();
        input.touch(device, step);
        input.touch(device, TouchStep::Frame);
    }
}

fn slot(id: WlcsTouchId) -> u32 {
    id.0
}

fn code(button: Button) -> ButtonCode {
    ButtonCode(button.0)
}

fn input_edge(edge: Edge) -> InputEdge {
    match edge {
        Edge::Press => InputEdge::Press,
        Edge::Release => InputEdge::Release,
    }
}

fn whole(at: Position) -> LogicalPoint {
    LogicalPoint { x: at.x, y: at.y }
}

/// The pixel a fixed-point position is in (the model's absolute positions are whole pixels).
fn floor(at: FixedPoint) -> LogicalPoint {
    let (x, y) = at.to_f64();
    LogicalPoint {
        x: x.floor() as i32,
        y: y.floor() as i32,
    }
}

/// A fixed-point delta in the model's sub-pixels (120 to the pixel).
fn sub(by: FixedPoint) -> SubPoint {
    let axis = |v: Fixed| SubPx((v.to_f64() * 120.0).round() as i32);
    SubPoint {
        x: axis(by.x),
        y: axis(by.y),
    }
}
