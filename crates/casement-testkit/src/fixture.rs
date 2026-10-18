//! The fixture: the real `State` on the headless backend, a Wayland display, test clients on
//! socket pairs, pumped on one thread until nothing moves.
//!
//! Mechanics frozen from SP3: each client's end of its socket pair is non-blocking; the
//! display's fd is a duplicate of its poll fd inside a calloop source (no `unsafe`); quiescence
//! is a round in which no server message was dispatched, no timer fired, no client event was
//! dispatched and no request was newly queued; `pump` stamps the virtual now into every
//! transcript; `advance` pumps at the current instant before the first stop, then stops at
//! every due instant, so chopping an advance differently gives the same transcript.

mod handout;
mod live;
mod standing;
mod timing;

use standing::arriving_standing;
pub use standing::{ASKED_PROGRAM, program_standing, trusted_standing};

use crate::fakes::{
    FakeA11y, FakePalmrest, FakeSession, FakeShellCalls, FakeSpawner, MemoryLockMarker,
    MemorySpaceStore,
};
use crate::input::FixtureInput;
use crate::spec::FixtureSpec;
use casement::cli::{
    ClockChoice, ConfigHome, Entropy, StartConfig, StartMode, TtyMode, XwaylandSource,
};
use casement::display::{self, SharedDisplay, attach_display};
use casement::icons::IconSearch;
use casement::privacy::{FilePrivacyStore, SystemDir};
use casement::seams::Seams;
use casement::space_store::StateHome;
use casement::time::{ClockSource, SharedClock};
use casement::xwayland::XwaylandConfig;
use casement::{State, core_loop};
use casement_backend::tty::TtySeams;
use casement_backend::{BackendChoice, BackendKind, FramePixels, HeadlessBackend, TtyBackend};
use casement_core::geometry::LogicalRect;
use casement_core::policy::ConnectionRole;
use casement_core::privilege::ClientStanding;
use casement_core::time::{Mono, Stop, TimerKey};
use casement_core::{Event, Model, OutputName, WindowId};
use casement_testclient::{ClientError, Pumped, TestClient};
use casement_wayland::consent::ImageWatch;
use casement_wayland::dmabuf::DmabufOffer;
use smithay::reexports::calloop::EventLoop;
use smithay::reexports::wayland_server::{Client, Display};
use std::collections::BTreeMap;
use std::os::fd::OwnedFd;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

/// One running compositor and its clients.
pub struct Fixture {
    state: State,
    display: SharedDisplay,
    event_loop: EventLoop<'static, State>,
    clients: BTreeMap<ClientId, TestClient>,
    /// Each client as the compositor holds it.
    servers: BTreeMap<ClientId, Client>,
    /// The next client's number.
    next_client: u32,
    /// The Spaces file the compositor stores into (a clone of the one it holds).
    space_store: MemorySpaceStore,
    /// The session lock's marker the compositor keeps (a clone of the one it holds).
    lock_marker: MemoryLockMarker,
    /// The D-Bus calls bindings made (a clone of the one the compositor holds).
    shell_calls: FakeShellCalls,
    /// The programs the compositor started (a clone of the one it holds).
    spawner: FakeSpawner,
    /// The accessibility roots clients claimed (a clone of the checker the compositor holds).
    a11y: FakeA11y,
    /// The session the headless backend never opens a device through: the fixture holds it so a
    /// suite can drive a seat change against the same clock (a tty backend takes its own).
    session: FakeSession,
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
#[non_exhaustive]
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

/// How many rounds `pump` runs before it calls the compositor runaway.
const ROUND_BOUND: u32 = 100_000;

/// How far ahead `frames` looks for the vblanks it waits on.
const FRAMES_HORIZON: Duration = Duration::from_secs(3600);

/// What the counters read at one instant (an advance reports the differences).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Tally {
    frames: u64,
    callbacks: u64,
}

impl Fixture {
    /// casement as `spec` describes it, pumped until it is quiet: outputs plugged, no client.
    pub fn start(spec: FixtureSpec) -> Result<Fixture, FixtureError> {
        Fixture::start_on(spec, BackendChoice::Headless, None)
    }

    /// casement on the tty backend over `seams` (the fakes of a [`TtyWorld`](crate::TtyWorld)),
    /// pumped until it is quiet: the outputs the fake devices report are plugged, no client.
    /// `spec`'s outputs and renderer are the headless backend's and are ignored; its settings,
    /// space store and ack policy apply. The helpers that plug virtual outputs and inject
    /// virtual input are headless only: a tty seat's outputs come from its KMS fake and its
    /// input from its [`FakeInputFeed`](crate::fakes::FakeInputFeed).
    pub fn start_tty(spec: FixtureSpec, seams: TtySeams) -> Result<Fixture, FixtureError> {
        Fixture::start_on(spec, BackendChoice::Tty, Some(seams))
    }

    fn start_on(
        spec: FixtureSpec,
        backend: BackendChoice,
        tty: Option<TtySeams>,
    ) -> Result<Fixture, FixtureError> {
        let event_loop: EventLoop<'static, State> =
            EventLoop::try_new().map_err(|error| FixtureError::Loop(error.to_string()))?;
        let handle = event_loop.handle();
        let mut display: Display<State> =
            Display::new().map_err(|error| FixtureError::Loop(error.to_string()))?;
        let outputs = match backend {
            BackendChoice::Tty => Vec::new(),
            _ => spec.outputs,
        };
        let config = StartConfig {
            backend,
            clock: ClockChoice::Virtual,
            outputs,
            renderer: spec.renderer,
            dmabuf: spec.dmabuf,
            cost: spec.cost,
            mode: StartMode::Normal,
            config_home: ConfigHome::At(scratch_config_home()),
            entropy: Entropy::Fixed(FIXTURE_ENTROPY),
            state_home: StateHome::Ephemeral,
            system_dir: SystemDir::Etc,
            xwayland: XwaylandConfig::off(),
            xwayland_source: XwaylandSource::Flag,
            tty: TtyMode::Seat,
            icons: IconSearch::Off,
        };
        let space_store = spec.space_store.clone();
        let lock_marker = spec.lock_marker.clone();
        let shell_calls = FakeShellCalls::new();
        let spawner = FakeSpawner::new();
        let a11y = FakeA11y::new();
        let seams = Seams::new(
            Box::new(space_store.clone()),
            Box::new(shell_calls.clone()),
            Box::new(spawner.clone()),
        )
        .with_lock_marker(Box::new(lock_marker.clone()))
        .with_wall(spec.wall)
        .with_a11y(Box::new(a11y.clone()));
        let kept_privacy = spec.privacy.is_some();
        let seams = match spec.privacy {
            Some(files) => seams.with_privacy(Box::new(FilePrivacyStore::new(files))),
            None => seams,
        };
        let seams = match tty {
            Some(tty) => seams.with_tty(tty),
            None => seams,
        };
        let mut state = State::with_seams(config, seams, &mut display, &handle)?;
        state.ack_policy = spec.ack_policy;
        if kept_privacy {
            // What the root does before it takes the first client in.
            casement::privacy::start(&mut state);
        }
        let display = attach_display(display, &handle)?;
        core_loop::announce_outputs(&mut state);
        if *state.settings() != spec.settings {
            core_loop::feed(
                &mut state,
                vec![Event::SettingsChanged(Box::new(spec.settings))],
            );
        }
        let mut fixture = Fixture {
            state,
            display,
            event_loop,
            clients: BTreeMap::new(),
            servers: BTreeMap::new(),
            next_client: 1,
            space_store,
            lock_marker,
            shell_calls,
            spawner,
            a11y,
            session: FakeSession::default(),
        };
        fixture.keep_timing_lines();
        fixture.pump();
        Ok(fixture)
    }

    /// The Spaces file the compositor stores into: read it back, or start another fixture on a
    /// clone of it to restart.
    pub fn space_store(&self) -> &MemorySpaceStore {
        &self.space_store
    }

    /// The session lock's marker the compositor keeps: read it back, or start another fixture
    /// on a clone of it to restart while locked.
    pub fn lock_marker(&self) -> &MemoryLockMarker {
        &self.lock_marker
    }

    /// The D-Bus calls bindings made.
    pub fn shell_calls(&self) -> &FakeShellCalls {
        &self.shell_calls
    }

    /// The programs the compositor started.
    pub fn spawner(&self) -> &FakeSpawner {
        &self.spawner
    }

    /// The administrator's floor changed on disk and casement was told to read it again
    /// (`casementctl privacy reload`).
    pub fn reload_floor(&mut self) {
        casement::privacy::reload_floor(&mut self.state);
        self.pump();
    }

    /// The accessibility roots clients claimed, as queued for checking.
    pub fn a11y(&self) -> &FakeA11y {
        &self.a11y
    }

    /// The bus said what `root` came to for the claim of `window`'s client.
    pub fn a11y_verdict(&mut self, window: WindowId, root: casement_core::window::A11yRoot) {
        casement::a11y::adopt(
            &mut self.state,
            casement_ipc::a11y::A11yVerdict { window, root },
        );
        self.pump();
    }

    /// A new client arriving through a socket of `role`. A public client is taken to be a
    /// desktop service the table trusts ([`trusted_standing`]), so what it uses is not asked
    /// about; [`Fixture::connect_via_ask`] connects one that is.
    pub fn connect(&mut self, role: ConnectionRole) -> ClientId {
        self.join(role.clone(), arriving_standing(&role), None)
    }

    /// A new client of `role`, whose program is taken to be `standing` when it is given.
    fn join(
        &mut self,
        role: ConnectionRole,
        standing: Option<ClientStanding>,
        watch: Option<Arc<dyn ImageWatch>>,
    ) -> ClientId {
        let (server, client) = UnixStream::pair().expect("a socket pair for the test client");
        client
            .set_nonblocking(true)
            .expect("a non-blocking client end");
        server
            .set_nonblocking(true)
            .expect("a non-blocking server end");
        let held = self.insert(server, role, standing, watch);
        let id = ClientId(self.next_client);
        self.servers.insert(id, held);
        self.next_client += 1;
        let client = TestClient::over(OwnedFd::from(client)).expect("the test client connects");
        self.clients.insert(id, client);
        self.pump();
        id
    }

    /// Client `id` hangs up (its end of the socket closes), and the fixture pumps so the
    /// compositor sees it go.
    pub fn disconnect(&mut self, id: ClientId) {
        self.servers.remove(&id);
        let gone = self.clients.remove(&id);
        assert!(gone.is_some(), "no client {id:?} in this fixture");
        drop(gone);
        self.pump();
    }

    /// Client `id` as the compositor holds it.
    pub(crate) fn server_client(&self, id: ClientId) -> Option<&Client> {
        self.servers.get(&id)
    }

    /// Client `id`.
    pub fn client(&mut self, id: ClientId) -> &mut TestClient {
        self.clients
            .get_mut(&id)
            .unwrap_or_else(|| panic!("no client {id:?} in this fixture"))
    }

    /// Dispatches the server and every client at the current instant until nothing moves.
    pub fn pump(&mut self) -> Pumped {
        let mut moved = Pumped::Quiet;
        for _ in 0..ROUND_BOUND {
            if self.round() == Pumped::Quiet {
                return moved;
            }
            moved = Pumped::Moved;
        }
        panic!(
            "the fixture did not go quiet in {ROUND_BOUND} rounds: a loop between server and clients"
        )
    }

    /// One round at the current instant: input, the event loop (display and clock), a flush,
    /// every client.
    fn round(&mut self) -> Pumped {
        let before = (self.display.dispatched(), self.clock().delivered());
        let input = core_loop::turn(&mut self.state);
        self.event_loop
            .dispatch(Some(Duration::ZERO), &mut self.state)
            .expect("the event loop dispatches");
        display::flush(&self.display);
        let now = self.now();
        let clients_moved = self
            .clients
            .values_mut()
            .map(|client| {
                client.set_now(now);
                client.pump()
            })
            .fold(Pumped::Quiet, |all, one| match (all, one) {
                (Pumped::Quiet, Pumped::Quiet) => Pumped::Quiet,
                _ => Pumped::Moved,
            });
        let after = (self.display.dispatched(), self.clock().delivered());
        match (input, before == after, clients_moved) {
            (0, true, Pumped::Quiet) => Pumped::Quiet,
            _ => Pumped::Moved,
        }
    }

    /// Moves the clock `by`, stopping at every timer and vblank on the way and pumping at each.
    pub fn advance(&mut self, by: Duration) -> Advanced {
        self.pump();
        let start = self.tally();
        let target = self.clock().now().after(by);
        while self.step(target) != Stop::End {}
        self.advanced_since(start)
    }

    /// Advances until `output` has presented `n` more frames.
    pub fn frames(&mut self, output: &OutputName, n: FrameCount) -> Advanced {
        self.pump();
        let start = self.tally();
        let goal = self.presented(output).saturating_add(u64::from(n.0));
        let horizon = self.clock().now().after(FRAMES_HORIZON);
        while self.presented(output) < goal && self.step(horizon) != Stop::End {}
        self.advanced_since(start)
    }

    /// Advances until `predicate` holds on the model, at most `bound` of virtual time.
    pub fn run_until(&mut self, predicate: impl Fn(&Model) -> bool, bound: Duration) -> Advanced {
        self.pump();
        let start = self.tally();
        let target = self.clock().now().after(bound);
        while !predicate(self.model()) && self.step(target) != Stop::End {}
        self.advanced_since(start)
    }

    /// The clock moves to its next stop at or before `limit` and the fixture pumps there.
    fn step(&mut self, limit: Mono) -> Stop {
        let clock = self.clock();
        let (at, stop) = clock.next_stop(limit);
        clock.advance_to(at);
        self.pump();
        stop
    }

    /// The virtual clock.
    fn clock(&self) -> SharedClock {
        match &self.state.clock {
            ClockSource::Virtual(clock) | ClockSource::Stepped(clock) => clock.clone(),
            ClockSource::Monotonic(_) => panic!("a fixture runs on the virtual clock"),
        }
    }

    /// The headless backend.
    pub(crate) fn headless(&self) -> &HeadlessBackend {
        match &self.state.backend {
            BackendKind::Headless(backend) => backend,
            _ => panic!("a fixture runs the headless backend"),
        }
    }

    /// The headless backend, mutably.
    pub(crate) fn headless_mut(&mut self) -> &mut HeadlessBackend {
        match &mut self.state.backend {
            BackendKind::Headless(backend) => backend,
            _ => panic!("a fixture runs the headless backend"),
        }
    }

    /// The compositor's state.
    pub(crate) fn state(&self) -> &State {
        &self.state
    }

    /// The instant on the virtual clock (the model's own `now` is the instant of its latest
    /// step, which a vblank or a timer that moves nothing in the model does not take).
    pub fn now(&self) -> Mono {
        self.clock().now()
    }

    /// The pixels of `output`'s last presented frame.
    pub(crate) fn last_pixels(&self, output: &OutputName) -> Option<&FramePixels> {
        self.headless()
            .last_frame(output)
            .map(|frame| &frame.pixels)
    }

    /// The compositor's state, mutably (the output helpers act on it).
    pub(crate) fn state_mut(&mut self) -> &mut State {
        &mut self.state
    }

    /// The counters now (on a tty seat: frames queued, and no frame callback counter).
    fn tally(&self) -> Tally {
        if let BackendKind::Tty(tty) = &self.state.backend {
            return Tally {
                frames: tty.frames_queued(),
                callbacks: 0,
            };
        }
        let backend = self.headless();
        Tally {
            frames: backend.frames_presented(),
            callbacks: backend.callbacks_sent(),
        }
    }

    /// Frames `output` has presented.
    fn presented(&self, output: &OutputName) -> u64 {
        match &self.state.backend {
            BackendKind::Tty(tty) => tty
                .scanouts
                .get(output)
                .map_or(0, |scanout| scanout.tally().presented),
            _ => self
                .headless()
                .outputs()
                .get(output)
                .map_or(0, |output| output.presented),
        }
    }

    /// What happened since `start`.
    fn advanced_since(&self, start: Tally) -> Advanced {
        let now = self.tally();
        let count = |later: u64, earlier: u64| u32::try_from(later - earlier).unwrap_or(u32::MAX);
        Advanced {
            frames: count(now.frames, start.frames),
            callbacks: count(now.callbacks, start.callbacks),
            timers: self.clock().take_fired(),
        }
    }

    /// The virtual input devices.
    pub fn input(&mut self) -> FixtureInput<'_> {
        FixtureInput::of(self)
    }

    /// The fake palmrest feed (bodies at P3).
    pub fn palmrest(&mut self) -> &mut FakePalmrest {
        todo!("P3-Z: Fixture::palmrest")
    }

    /// The fake session the fixture holds (the headless backend opens no device through it).
    pub fn session(&mut self) -> &mut FakeSession {
        &mut self.session
    }

    /// The last frames' element tree, with what a leak would leave behind: the outputs the
    /// model, the backend, the clock, the protocol module and the trackers each still hold.
    pub fn scene(&self) -> SceneDump {
        let outputs: Vec<serde_json::Value> = self
            .model()
            .outputs()
            .0
            .iter()
            .map(|(key, state)| {
                serde_json::json!({
                    "key": key.as_str(),
                    "name": state.facts.name.as_str(),
                    "mode": [state.config.mode.size.w, state.config.mode.size.h],
                    "scale": state.config.scale.0,
                    "at": [state.config.position.x, state.config.position.y],
                })
            })
            .collect();
        let frame = core_loop::sample_scene(&self.state);
        SceneDump(serde_json::json!({
            "outputs": outputs,
            "frame": frame.text(),
            "held": {
                "backend_outputs": self.headless().outputs().len(),
                "clock_outputs": self.clock().output_count(),
                "wayland_outputs": self.state.wl.output.outputs.len(),
                "damage_trackers": self.state.render.trackers.len(),
            },
        }))
    }

    /// The settings file changed to `doc`, as the watch on it tells the loop.
    pub fn settings_file_changed(&mut self, doc: &casement::config::CasementDoc) {
        casement::settings_live::apply(&mut self.state, doc);
    }

    /// The model.
    pub fn model(&self) -> &Model {
        &self.state.core.model
    }

    /// The dma-buf offer the compositor made (none until a main device is offered).
    pub fn dmabuf_offer(&self) -> Option<&DmabufOffer> {
        self.state.wl.dmabuf.offered.as_ref()
    }

    /// The tty backend, on a fixture started with [`start_tty`](Fixture::start_tty).
    pub fn tty(&self) -> &TtyBackend {
        match &self.state.backend {
            BackendKind::Tty(tty) => tty,
            _ => panic!("this fixture runs the headless backend"),
        }
    }
}

/// The entropy Space ids are minted from: fixed, so two runs mint the same ids.
const FIXTURE_ENTROPY: [u8; 16] = *b"casement-fixture";

/// A settings directory that does not exist (the document is missing: the defaults).
fn scratch_config_home() -> PathBuf {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("casement-fixture-{}-{n}", std::process::id()))
}
