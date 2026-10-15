//! The tty backend end to end (P1-T): the real `State` started on a seat made of fakes, with
//! two outputs lit, frames flowing, a VT switch away and back, hotplug, DPMS, gamma, capture,
//! input, and a suspend heard through the fake logind on a private bus.

use casement::suspend;
use casement_backend::GpuId;
use casement_backend::gpu::SysRenderNodes;
use casement_backend::tty::TtySession;
use casement_backend::tty::{ConnectorId, SeatState};
use casement_core::input::{
    Argv, Binding, BindingAction, Bindings, Chord, DeviceClass, Edge, InputKind, KeyCode,
};
use casement_core::output::OutputPower;
use casement_core::policy::ConnectionRole;
use casement_core::{DeviceId, OutputName};
use casement_ipc::sleep::SleepBus;
use casement_testclient::Pumped;
use casement_testclient::capture::{BufferAsk, CursorMode, OutputCapture};
use casement_testclient::manage::gamma::{ControlState as GammaState, GammaControls};
use casement_testclient::manage::output_power::{
    ControlState as PowerState, OutputPowers, PowerMode,
};
use casement_testkit::fakes::{
    FakeKms, FakeLogind, GlassHandle, PrivateBus, SleepPhase, SyntheticTopology,
};
use casement_testkit::gpu::{
    CrossImport, FabricatedSysRoot, FormatSet, GpuTopology, SoftEngine, SoftGpu, VendorFact,
};
use casement_testkit::{Fixture, FixtureSpec, FrameCount, TtyWorld};
use casement_wayland::dmabuf::{MainDevice, OfferUse};
use std::time::Duration;

const KEYBOARD: DeviceId = DeviceId(1);
const KEY_LEFTCTRL: KeyCode = KeyCode(29);
const KEY_LEFTALT: KeyCode = KeyCode(56);
const KEY_F3: KeyCode = KeyCode(61);
const KEY_LEFTMETA: KeyCode = KeyCode(125);
const KEY_SPACE: KeyCode = KeyCode(57);

fn name(text: &str) -> OutputName {
    OutputName::parse(text).expect("a connector name")
}

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

/// One GPU that drives every connector of the dock.
fn gpus() -> GpuTopology {
    let drives = ["eDP-1", "DP-1", "HDMI-A-1"].map(name).to_vec();
    GpuTopology {
        gpus: vec![
            SoftGpu::new(
                GpuId(0),
                SoftEngine::Pixman,
                FormatSet::LinearOnly,
                CrossImport::Direct,
                VendorFact::Amd,
            )
            .driving(drives),
        ],
    }
}

/// A seat over a dock of three connectors with the HDMI monitor unplugged (two outputs), the
/// compositor started on it with `spec`.
struct Seat {
    _tree: tempfile::TempDir,
    world: TtyWorld,
    fixture: Fixture,
}

fn seat_with(kms: FakeKms, spec: FixtureSpec) -> Seat {
    let tree = tempfile::tempdir().expect("a directory for the fabricated /sys");
    let topology = gpus();
    let root = FabricatedSysRoot::build(&topology, tree.path()).expect("a fabricated tree");
    let world = TtyWorld::new(kms);
    let seams = world.seams(
        Box::new(SysRenderNodes::new(root)),
        Box::new(topology),
        casement_core::gpu::GpuChoice::Auto,
    );
    let fixture = Fixture::start_tty(spec, seams).expect("the compositor starts on the seat");
    Seat {
        _tree: tree,
        world,
        fixture,
    }
}

fn two_monitors() -> FakeKms {
    let kms = FakeKms::synthetic(SyntheticTopology::Dock);
    kms.unplug(ConnectorId(3));
    kms
}

fn seat() -> Seat {
    seat_with(two_monitors(), FixtureSpec::desktop())
}

fn glass(world: &TtyWorld, connector: u32) -> GlassHandle {
    world
        .kms
        .glass(ConnectorId(connector))
        .expect("a scanout surface was handed out")
}

fn power_of(fixture: &Fixture, connector: &str) -> OutputPower {
    let wanted = name(connector);
    fixture
        .model()
        .outputs()
        .0
        .values()
        .find(|state| state.facts.name == wanted)
        .map(|state| state.config.power)
        .expect("a plugged output")
}

#[test]
fn the_seat_starts_with_both_outputs_lit_and_a_scanout_each() {
    let seat = seat();
    assert_eq!(seat.fixture.model().outputs().0.len(), 2);
    let lit = seat.world.kms.world().lit.clone();
    let preferred = |connector: u32| lit.get(&ConnectorId(connector)).map(|o| o.mode.size);
    assert_eq!(preferred(1).map(|s| (s.w, s.h)), Some((2880, 1800)));
    assert_eq!(preferred(2).map(|s| (s.w, s.h)), Some((3840, 2160)));
    assert!(
        !lit.contains_key(&ConnectorId(3)),
        "the unplugged monitor is dark"
    );
    assert_eq!(
        seat.fixture.tty().scanouts.names(),
        [name("DP-1"), name("eDP-1")]
    );
    let mut modeset: Vec<_> = seat
        .world
        .kms
        .commits()
        .iter()
        .flat_map(|commit| commit.outputs.iter().map(|output| output.connector))
        .collect();
    modeset.sort();
    modeset.dedup();
    assert_eq!(modeset, [ConnectorId(1), ConnectorId(2)], "a modeset each");
}

#[test]
fn frames_are_queued_on_every_lit_output_and_none_is_late() {
    let mut seat = seat();
    let advanced = seat.fixture.advance(ms(100));
    assert!(advanced.frames > 0);
    // The panel at 120 Hz, the monitor at 60.
    assert!(glass(&seat.world, 1).queued() >= 10);
    assert!(glass(&seat.world, 2).queued() >= 5);
    for output in ["eDP-1", "DP-1"] {
        let tally = seat
            .fixture
            .tty()
            .scanouts
            .get(&name(output))
            .map(|scanout| scanout.tally())
            .expect("lit");
        assert!(tally.presented > 0, "{output} presented frames");
        assert_eq!(tally.missed, 0, "{output} missed no vblank");
    }
}

#[test]
fn clients_are_offered_the_render_plans_dmabuf_feedback() {
    let seat = seat();
    let plan = seat.fixture.tty().plan.clone().expect("a render plan");
    assert_eq!(plan.scanout.len(), 3, "the GPU drives all three connectors");
    let offer = seat
        .fixture
        .dmabuf_offer()
        .expect("a main device is offered");
    let render = MainDevice::of(226, 128);
    assert_eq!(offer.main_device, render);
    let shape: Vec<_> = offer
        .tranches
        .iter()
        .map(|tranche| (tranche.device, tranche.purpose, tranche.formats.len()))
        .collect();
    assert_eq!(
        shape,
        [
            (render, OfferUse::Scanout, 1),
            (render, OfferUse::Sampling, 1)
        ],
        "the plane's XRGB8888 first, then ARGB8888 for compositing"
    );
}

fn type_chord(seat: &mut Seat, codes: &[KeyCode], edge: Edge) {
    for code in codes {
        seat.world
            .input
            .push(KEYBOARD, InputKind::Key { code: *code, edge });
    }
    seat.fixture.pump();
}

#[test]
fn a_vt_chord_pauses_the_seat_and_the_return_lights_it_again() {
    let mut seat = seat();
    seat.world
        .input
        .plug(KEYBOARD, DeviceClass::Keyboard, "keyboard");
    seat.fixture.pump();
    seat.fixture.advance(ms(50));
    let (panel, monitor) = (glass(&seat.world, 1), glass(&seat.world, 2));
    let before = (panel.queued(), monitor.queued());
    let commits = seat.world.kms.commits().len();

    // Ctrl+Alt+F3 is `XF86Switch_VT_3` after xkb: the seat's, not a client's, not a binding's.
    type_chord(&mut seat, &[KEY_LEFTCTRL, KEY_LEFTALT, KEY_F3], Edge::Press);
    assert_eq!(seat.world.session.switched_to(), Some(3));
    assert_eq!(seat.fixture.tty().seat, SeatState::Lent);
    assert_eq!(seat.world.kms.seat_changes(), (1, 0));
    assert_eq!(seat.world.input.suspends(), 1);
    type_chord(
        &mut seat,
        &[KEY_F3, KEY_LEFTALT, KEY_LEFTCTRL],
        Edge::Release,
    );

    seat.fixture.advance(ms(100));
    assert_eq!(
        (panel.queued(), monitor.queued()),
        before,
        "no frame is queued while another VT has the seat"
    );

    seat.world.session.resume();
    seat.fixture.pump();
    assert_eq!(seat.fixture.tty().seat, SeatState::Held);
    assert_eq!(seat.world.kms.seat_changes(), (1, 1));
    assert_eq!(seat.world.input.resumes(), 1);
    assert_eq!((panel.resets(), monitor.resets()), (1, 1));
    assert_eq!(
        seat.world.kms.commits().len(),
        commits + 2,
        "every output is committed again"
    );
    seat.fixture.advance(ms(100));
    assert!(panel.queued() > before.0 && monitor.queued() > before.1);
}

#[test]
fn a_key_that_is_not_a_vt_chord_switches_nothing() {
    let mut seat = seat();
    seat.world
        .input
        .plug(KEYBOARD, DeviceClass::Keyboard, "keyboard");
    seat.fixture.pump();
    type_chord(&mut seat, &[KEY_LEFTCTRL, KEY_F3], Edge::Press);
    assert_eq!(seat.world.session.switched_to(), None);
    assert_eq!(seat.fixture.tty().seat, SeatState::Held);
}

#[test]
fn hotplug_removes_and_adds_a_scanout() {
    let mut seat = seat();
    seat.fixture.advance(ms(50));
    seat.world.kms.unplug(ConnectorId(2));
    seat.world.changed();
    seat.fixture.pump();
    assert_eq!(seat.fixture.model().outputs().0.len(), 1);
    assert_eq!(seat.fixture.tty().scanouts.names(), [name("eDP-1")]);
    assert!(!seat.fixture.tty().lit.contains_key(&name("DP-1")));

    let monitor = SyntheticTopology::Dock.resources().connectors[1].clone();
    seat.world.kms.plug(monitor);
    seat.world.changed();
    seat.fixture.pump();
    assert_eq!(seat.fixture.model().outputs().0.len(), 2);
    assert_eq!(
        seat.fixture.tty().scanouts.names(),
        [name("DP-1"), name("eDP-1")]
    );
    let again = glass(&seat.world, 2);
    seat.fixture.advance(ms(100));
    assert!(again.queued() >= 5, "the replugged monitor shows frames");
}

fn settle_power(fixture: &mut Fixture, powers: &mut OutputPowers) {
    fixture.pump();
    powers.pump().expect("pump");
    fixture.pump();
    powers.pump().expect("pump");
}

#[test]
fn dpms_off_darkens_the_output_and_on_queues_a_frame_at_once() {
    let mut seat = seat();
    let client = seat.fixture.connect(ConnectionRole::Public);
    seat.fixture.pump();
    let mut powers =
        OutputPowers::start(seat.fixture.client(client).globals()).expect("the manager");
    settle_power(&mut seat.fixture, &mut powers);
    let control = powers.control("eDP-1").expect("a control");
    settle_power(&mut seat.fixture, &mut powers);
    assert_eq!(
        powers.state(control),
        Some(PowerState::Reported(PowerMode::On))
    );
    seat.fixture.advance(ms(50));
    let panel = glass(&seat.world, 1);

    powers.set_mode(control, PowerMode::Off).expect("off");
    settle_power(&mut seat.fixture, &mut powers);
    assert_eq!(power_of(&seat.fixture, "eDP-1"), OutputPower::Off);
    assert_eq!(
        seat.world.kms.power_of(ConnectorId(1)),
        Some(OutputPower::Off)
    );
    assert_eq!(panel.blanks(), 1);
    let dark = panel.queued();
    seat.fixture.advance(ms(100));
    assert_eq!(panel.queued(), dark, "a dark output queues no frame");

    powers.set_mode(control, PowerMode::On).expect("on");
    settle_power(&mut seat.fixture, &mut powers);
    assert_eq!(
        seat.world.kms.power_of(ConnectorId(1)),
        Some(OutputPower::On)
    );
    assert!(
        panel.queued() > dark,
        "DPMS On queues a frame before the next vblank"
    );
}

fn settle_gamma(fixture: &mut Fixture, gamma: &mut GammaControls) {
    fixture.pump();
    gamma.pump().expect("pump");
    fixture.pump();
    gamma.pump().expect("pump");
}

#[test]
fn the_gamma_table_size_is_the_crtcs_and_a_ramp_reaches_the_device() {
    let mut resources = SyntheticTopology::Dock.resources();
    resources.crtcs[0] = resources.crtcs[0].clone().with_gamma_size(1024);
    let kms = FakeKms::with(resources);
    kms.unplug(ConnectorId(3));
    let mut seat = seat_with(kms, FixtureSpec::desktop());
    let client = seat.fixture.connect(ConnectionRole::Public);
    seat.fixture.pump();
    let mut gamma = GammaControls::start(seat.fixture.client(client).globals()).expect("manager");
    settle_gamma(&mut seat.fixture, &mut gamma);
    let control = gamma.control("eDP-1").expect("a control");
    settle_gamma(&mut seat.fixture, &mut gamma);
    let Some(GammaState::Sized(size)) = gamma.state(control) else {
        panic!("the control announces its size: {:?}", gamma.state(control));
    };
    assert_eq!(size, 1024, "the panel's CRTC has 1024 steps");

    let up: Vec<u16> = (0..size)
        .map(|n| u16::try_from(n * 64).unwrap_or(u16::MAX))
        .collect();
    gamma
        .set_ramp(control, &up, &up, &up)
        .expect("a ramp of the announced size");
    settle_gamma(&mut seat.fixture, &mut gamma);
    assert!(
        seat.world.kms.world().gamma.contains_key(&ConnectorId(1)),
        "the ramp was loaded on the panel's CRTC"
    );
}

#[test]
fn a_capture_of_a_kms_output_returns_its_frame() {
    let kms = FakeKms::synthetic(SyntheticTopology::Dock);
    let mut seat = seat_with(kms, FixtureSpec::desktop());
    let client = seat.fixture.connect(ConnectionRole::Public);
    seat.fixture.pump();
    let mut capture =
        OutputCapture::start(seat.fixture.client(client).globals()).expect("the managers");
    settle_capture(&mut seat.fixture, &mut capture);
    capture
        .open("HDMI-A-1", CursorMode::Hidden)
        .expect("a session");
    settle_capture(&mut seat.fixture, &mut capture);
    // The session turns the shadow on; two vblanks later it holds a frame.
    seat.fixture.frames(&name("HDMI-A-1"), FrameCount(2));
    capture.request(BufferAsk::Announced).expect("a frame");
    settle_capture(&mut seat.fixture, &mut capture);
    let frame = capture
        .take()
        .expect("the frame was answered")
        .expect("a frame, not a failure");
    assert_eq!((frame.size.w, frame.size.h), (1920, 1080));
}

fn settle_capture(fixture: &mut Fixture, capture: &mut OutputCapture) {
    for _ in 0..100 {
        let compositor = fixture.pump();
        let client = capture.pump().expect("pump the capture");
        if (compositor, client) == (Pumped::Quiet, Pumped::Quiet) {
            return;
        }
    }
    panic!("the compositor and the capture did not go quiet");
}

#[test]
fn a_mouse_and_a_touchpad_both_start_with_natural_scrolling() {
    let mut seat = seat_with(two_monitors(), FixtureSpec::desktop());
    let mouse = DeviceId(3);
    let touchpad = DeviceId(4);
    seat.world.input.plug(mouse, DeviceClass::Pointer, "mouse");
    seat.world
        .input
        .plug(touchpad, DeviceClass::Touchpad, "touchpad");
    seat.fixture.pump();
    let applied = seat.world.input.applied();
    let natural = |id: DeviceId| {
        applied
            .iter()
            .find(|(device, _)| *device == id)
            .map(|(_, settings)| settings.natural_scroll)
    };
    assert_eq!(natural(mouse), Some(casement_core::settings::Enabled::On));
    assert_eq!(
        natural(touchpad),
        Some(casement_core::settings::Enabled::On)
    );
}

/// A change in the settings file reaches the devices that are already there, each by its own
/// profile, and not a keyboard; an unrelated change tells nobody; a device that left is not told.
#[test]
fn a_settings_change_reaches_a_device_already_present() {
    use casement::config::{CasementDoc, PointerSection, TouchpadSection};
    use casement_core::settings::Enabled;
    let mut seat = seat_with(two_monitors(), FixtureSpec::desktop());
    let (mouse, touchpad, gone) = (DeviceId(3), DeviceId(4), DeviceId(5));
    seat.world.input.plug(mouse, DeviceClass::Pointer, "mouse");
    seat.world
        .input
        .plug(touchpad, DeviceClass::Touchpad, "touchpad");
    seat.world
        .input
        .plug(gone, DeviceClass::Pointer, "other mouse");
    seat.world
        .input
        .plug(KEYBOARD, DeviceClass::Keyboard, "keyboard");
    seat.fixture.pump();
    seat.world.input.unplug(gone);
    seat.fixture.pump();
    let before = seat.world.input.applied().len();

    let doc = CasementDoc {
        pointer: PointerSection {
            natural_scroll: Enabled::Off,
            ..PointerSection::default()
        },
        ..CasementDoc::default()
    };
    seat.fixture.settings_file_changed(&doc);
    let applied = seat.world.input.applied();
    assert_eq!(applied.len(), before + 1, "only the mouse is told");
    let (device, settings) = applied.last().expect("an application");
    assert_eq!(*device, mouse);
    assert_eq!(settings.natural_scroll, Enabled::Off);

    seat.fixture.settings_file_changed(&doc);
    assert_eq!(
        seat.world.input.applied().len(),
        before + 1,
        "the same file again changes nothing"
    );

    let doc = CasementDoc {
        touchpad: TouchpadSection {
            tap: Enabled::Off,
            ..TouchpadSection::default()
        },
        ..doc
    };
    seat.fixture.settings_file_changed(&doc);
    let applied = seat.world.input.applied();
    assert_eq!(applied.len(), before + 2, "then only the touchpad");
    assert_eq!(applied.last().map(|(device, _)| *device), Some(touchpad));
    assert_eq!(applied.last().map(|(_, s)| s.tap), Some(Enabled::Off));
}

#[test]
fn input_devices_are_configured_and_their_keys_reach_the_bindings() {
    let mut spec = FixtureSpec::desktop();
    let argv = Argv(vec!["foot".to_owned()]);
    spec.settings.bindings = Bindings(vec![Binding {
        chord: Chord::parse("cmd+space").expect("a chord"),
        action: BindingAction::Spawn(argv.clone()),
    }]);
    let mut seat = seat_with(two_monitors(), spec);
    let touchpad = DeviceId(2);
    seat.world
        .input
        .plug(touchpad, DeviceClass::Touchpad, "touchpad");
    seat.world
        .input
        .plug(KEYBOARD, DeviceClass::Keyboard, "keyboard");
    seat.fixture.pump();
    let applied = seat.world.input.applied();
    assert_eq!(applied.len(), 1, "only the touchpad takes pointer settings");
    assert_eq!(applied[0].0, touchpad);
    assert_eq!(
        applied[0].1.natural_scroll,
        casement_core::settings::Enabled::On,
        "the touchpad profile's default"
    );

    for edge in [Edge::Press, Edge::Release] {
        let codes = [KEY_LEFTMETA, KEY_SPACE];
        type_chord(&mut seat, &codes, edge);
    }
    let started = seat.fixture.spawner().started();
    assert_eq!(started.len(), 1, "the chord reached the bindings");
    assert_eq!(started[0].argv, argv);
}

/// How many times `until` looks before it gives up (each look pumps the compositor and yields
/// the processor: the fake logind and the watcher are threads, and a test never sleeps).
const LOOKS: u32 = 2_000_000;

/// Pumps the compositor, yielding to the bus threads, until `done` holds.
fn until(fixture: &mut Fixture, what: &str, done: impl Fn(&Fixture) -> bool) {
    for _ in 0..LOOKS {
        fixture.pump();
        if done(fixture) {
            return;
        }
        std::thread::yield_now();
    }
    panic!("gave up waiting until {what}");
}

#[test]
fn a_whole_session_from_start_to_suspend_and_back() {
    // Start: two outputs announced, lit, frames flowing.
    let mut seat = seat();
    seat.world
        .input
        .plug(KEYBOARD, DeviceClass::Keyboard, "keyboard");
    seat.fixture.pump();
    assert_eq!(seat.fixture.model().outputs().0.len(), 2);
    assert!(seat.fixture.advance(ms(100)).frames > 0);
    let panel = glass(&seat.world, 1);

    // A VT switch away and back.
    type_chord(&mut seat, &[KEY_LEFTCTRL, KEY_LEFTALT, KEY_F3], Edge::Press);
    assert_eq!(seat.fixture.tty().seat, SeatState::Lent);
    seat.world.session.resume();
    seat.fixture.pump();
    assert_eq!(seat.fixture.tty().seat, SeatState::Held);

    // Hotplug: the third monitor.
    let hdmi = SyntheticTopology::Dock.resources().connectors[2].clone();
    seat.world.kms.plug(hdmi);
    seat.world.changed();
    seat.fixture.pump();
    assert_eq!(seat.fixture.model().outputs().0.len(), 3);

    // Suspend: logind says so on a private bus, the watcher pushes it into the session, the
    // backend lets go of its devices and only then does logind's inhibitor go.
    let bus = PrivateBus::start().expect("a private bus");
    let logind = FakeLogind::serve(bus.address()).expect("the fake logind");
    let mut session = seat.world.session.clone();
    let link = session.sleep_link().expect("the session hears a suspend");
    let _watch = suspend::watch(link, SleepBus::At(bus.address().to_owned()))
        .expect("the watcher subscribes");
    assert_eq!(logind.held_inhibitors(), ["sleep"]);
    let (pauses, resumes) = seat.world.kms.seat_changes();

    logind
        .prepare_for_sleep(SleepPhase::Starting)
        .expect("PrepareForSleep(true)");
    until(&mut seat.fixture, "the seat is lent for the sleep", |f| {
        f.tty().seat == SeatState::Lent
    });
    logind
        .wait_released(Duration::from_secs(5))
        .expect("the inhibitor is released once the backend let go");
    assert_eq!(seat.world.kms.seat_changes(), (pauses + 1, resumes));
    let queued = panel.queued();
    seat.fixture.advance(ms(50));
    assert_eq!(panel.queued(), queued, "nothing is shown while asleep");

    logind
        .prepare_for_sleep(SleepPhase::Resumed)
        .expect("PrepareForSleep(false)");
    until(&mut seat.fixture, "the seat is back", |f| {
        f.tty().seat == SeatState::Held
    });
    assert_eq!(seat.world.kms.seat_changes(), (pauses + 1, resumes + 1));
    assert_eq!(
        seat.world.session.settled_notes().len(),
        4,
        "the VT pause and return and the sleep pause and return were each acted on"
    );
    seat.fixture.advance(ms(100));
    assert!(panel.queued() > queued, "frames flow after the wake");
    until(&mut seat.fixture, "the inhibitor is taken again", |_| {
        logind.held_inhibitors() == ["sleep"]
    });
}
