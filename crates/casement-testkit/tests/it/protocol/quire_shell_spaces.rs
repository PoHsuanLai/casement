//! `quire_shell_v1` version 2, the Spaces bar's three asks: which outputs Mission Control is up
//! on (`overview_output`), a picture of a Space (`get_space_capture_source`), and dropping a
//! dragged window on a Space (`space_target`, `drop_target`). Shell connections only. Version 3
//! (the pictures of Spaces that are not shown, the new-Space tile, a drag to another output)
//! is in `quire_shell_drop.rs`, which builds on this file's desk.

use crate::protocol::image_capture::{settle as settle_capture, small};
use crate::protocol::xdg_shell::{map_toplevel, pointer, windows};
use casement_core::geometry::{LogicalPoint, PhysicalSize};
use casement_core::input::{ButtonCode, Edge};
use casement_core::overview::Overview;
use casement_core::policy::ConnectionRole;
use casement_core::window::SpacePlace;
use casement_core::{DeviceId, OutputKey, WindowId};
use casement_layout::{LayoutKnobs, OverviewSlot, RustScene, SceneLayout, spec_of};
use casement_testclient::capture::{BufferAsk, OutputCapture, Size};
use casement_testclient::quire::desktop::DesktopWatch;
use casement_testclient::quire::shell::{BoundOutput, OverviewMode, ShellHandle, Shown, Toggle};
use casement_testclient::surface::toplevel::{Toplevel, WmBase};
use casement_testclient::{Pattern, shm_buffer};
use casement_testkit::{ClientId, Fixture, FixtureSpec};
use porter_core::SpaceId;
use std::time::Duration;

/// The left button (`BTN_LEFT`).
const LEFT: ButtonCode = ButtonCode(0x110);

/// A one-output desktop with two windows, a version 4 shell with its outputs bound, a pointer
/// and a second Space on the output.
pub(super) struct Desk {
    pub(super) fixture: Fixture,
    pub(super) shell: ShellHandle,
    pub(super) shell_id: ClientId,
    pub(super) outputs: Vec<BoundOutput>,
    pub(super) windows: Vec<WindowId>,
    pub(super) pointer: DeviceId,
    pub(super) second: SpaceId,
    pub(super) app: ClientId,
    pub(super) tops: Vec<Toplevel>,
    _base: WmBase,
    _watch: DesktopWatch,
}

pub(super) fn desk() -> Desk {
    let mut fixture = Fixture::start(small()).expect("fixture");
    let app = fixture.connect(ConnectionRole::Public);
    let base = WmBase::bind(fixture.client(app), 7);
    let tops = vec![
        map_toplevel(&mut fixture, app, &base, (320, 200)),
        map_toplevel(&mut fixture, app, &base, (240, 160)),
    ];
    let shell_id = fixture.connect(ConnectionRole::Shell);
    let shell = ShellHandle::bind(fixture.client(shell_id), 4);
    let outputs = ShellHandle::bind_outputs(fixture.client(shell_id));
    let mut watch = DesktopWatch::start(fixture.client(shell_id).globals()).expect("the manager");
    for _ in 0..3 {
        fixture.pump();
        watch.pump().expect("pump");
    }
    watch.create_space().expect("the request is sent");
    for _ in 0..3 {
        fixture.pump();
        watch.pump().expect("pump");
    }
    let output = fixture
        .model()
        .outputs()
        .0
        .keys()
        .next()
        .cloned()
        .expect("an output");
    let second = fixture.model().spaces().row(&output)[1].id.clone();
    let pointer = pointer(&mut fixture);
    let windows = windows(&fixture);
    Desk {
        fixture,
        shell,
        shell_id,
        outputs,
        windows,
        pointer,
        second,
        app,
        tops,
        _base: base,
        _watch: watch,
    }
}

pub(super) fn output_key(fixture: &Fixture) -> OutputKey {
    fixture
        .model()
        .outputs()
        .0
        .keys()
        .next()
        .cloned()
        .expect("an output")
}

pub(super) fn open(desk: &mut Desk) {
    desk.shell
        .overview(OverviewMode::MissionControl, Toggle::On);
    desk.fixture.pump();
    desk.fixture.advance(Duration::from_millis(500));
}

pub(super) fn slots(fixture: &Fixture) -> Vec<OverviewSlot> {
    let model = fixture.model();
    RustScene
        .arrange(&spec_of(model), &LayoutKnobs::of(model.settings()))
        .overview
}

pub(super) fn centre_of(desk: &Desk, window: WindowId) -> LogicalPoint {
    slots(&desk.fixture)
        .iter()
        .find(|slot| slot.window == window)
        .expect("its slot")
        .rect
        .centre()
}

/// The shell puts one drop target in the top left corner, above every slot.
pub(super) fn target(desk: &mut Desk) {
    let output = &desk.outputs[0].object;
    desk.shell.space_target_clear(output);
    desk.shell
        .space_target(output, desk.second.as_str(), (10, 10), (100, 90));
    desk.fixture.pump();
}

pub(super) fn press(desk: &mut Desk) {
    desk.fixture.input().button(desk.pointer, LEFT, Edge::Press);
    desk.fixture.pump();
}

pub(super) fn release(desk: &mut Desk) {
    desk.fixture
        .input()
        .button(desk.pointer, LEFT, Edge::Release);
    desk.fixture.pump();
}

pub(super) fn move_to(desk: &mut Desk, at: LogicalPoint) {
    desk.fixture.input().pointer_to(desk.pointer, at);
    desk.fixture.pump();
}

/// Mission Control opens on every output at once: the shell hears `overview_output` for each
/// output before each state, shown while it is up and hidden when it is closed. A shell bound at
/// version 1 hears none.
#[test]
fn the_shell_hears_which_outputs_mission_control_is_up_on() {
    let spec = FixtureSpec::desktop().with_outputs(&[
        "HEADLESS-1:640x480@60000:120:0,0",
        "HEADLESS-2:640x480@60000:120:640,0",
    ]);
    let mut fixture = Fixture::start(spec).expect("fixture");
    let id = fixture.connect(ConnectionRole::Shell);
    let shell = ShellHandle::bind(fixture.client(id), 2);
    let outputs = ShellHandle::bind_outputs(fixture.client(id));
    let old_id = fixture.connect(ConnectionRole::Shell);
    let old = ShellHandle::bind(fixture.client(old_id), 1);
    fixture.pump();
    assert_eq!(outputs.len(), 2);
    shell.overview(OverviewMode::MissionControl, Toggle::On);
    fixture.pump();
    let entering = shell.overview_outputs();
    assert_eq!(entering.len(), 2, "one per output");
    assert!(
        entering
            .iter()
            .all(|(_, shown)| *shown == Some(Shown::Shown))
    );
    for bound in &outputs {
        assert!(
            entering.iter().any(|(output, _)| *output == bound.object),
            "{:?}",
            bound.connector()
        );
    }
    fixture.advance(Duration::from_millis(500));
    shell.overview(OverviewMode::MissionControl, Toggle::Off);
    fixture.pump();
    fixture.advance(Duration::from_millis(500));
    let all = shell.overview_outputs();
    assert!(
        all.iter()
            .rev()
            .take(2)
            .all(|(_, shown)| *shown == Some(Shown::Hidden))
    );
    assert!(
        all.len() >= 8,
        "entering, open, leaving, closed: {}",
        all.len()
    );
    assert!(
        old.overview_outputs().is_empty(),
        "version 1 hears nothing new"
    );
    assert!(!old.overview_states().is_empty(), "but still the states");
}

/// A press on a slot that stays put is the pick it always was.
#[test]
fn a_press_that_hardly_moves_still_picks() {
    let mut desk = desk();
    target(&mut desk);
    open(&mut desk);
    let window = desk.windows[0];
    let at = centre_of(&desk, window);
    move_to(&mut desk, at);
    press(&mut desk);
    move_to(
        &mut desk,
        LogicalPoint {
            x: at.x + 3,
            y: at.y,
        },
    );
    assert_eq!(desk.fixture.model().overview(), Overview::Open, "not yet");
    release(&mut desk);
    assert_eq!(desk.fixture.model().overview(), Overview::Leaving);
    assert_eq!(desk.fixture.model().focus().key, Some(window));
}

/// A window dragged over a target shows the Space under it to the shell; released there it moves
/// to that Space and Mission Control stays open.
#[test]
fn a_window_dropped_on_a_target_moves_to_that_space() {
    let mut desk = desk();
    target(&mut desk);
    open(&mut desk);
    let window = desk.windows[0];
    let at = centre_of(&desk, window);
    move_to(&mut desk, at);
    press(&mut desk);
    move_to(&mut desk, LogicalPoint { x: 50, y: 50 });
    let output = desk.outputs[0].object.clone();
    // One event per binding of the output: the client holds two, so look for this one's.
    assert!(
        desk.shell
            .drop_targets()
            .contains(&(output.clone(), Some(desk.second.as_str().to_owned()))),
        "the Space under the window is named"
    );
    let heard = desk.shell.drop_targets().len();
    release(&mut desk);
    assert_eq!(
        desk.fixture.model().windows()[&window].place,
        SpacePlace::On(desk.second.clone())
    );
    assert_eq!(
        desk.fixture.model().overview(),
        Overview::Open,
        "stays in Mission Control"
    );
    assert!(
        desk.shell.drop_targets()[heard..].contains(&(output.clone(), None)),
        "the drag's end clears the target"
    );
}

/// Released anywhere else the window goes back to its slot: it stays on its Space and Mission
/// Control stays open, and the shell is told the highlight is gone.
#[test]
fn a_window_released_elsewhere_flies_back() {
    let mut desk = desk();
    target(&mut desk);
    open(&mut desk);
    let window = desk.windows[0];
    let before = desk.fixture.model().windows()[&window].place.clone();
    let at = centre_of(&desk, window);
    move_to(&mut desk, at);
    press(&mut desk);
    move_to(&mut desk, LogicalPoint { x: 50, y: 50 });
    move_to(&mut desk, LogicalPoint { x: 560, y: 440 });
    release(&mut desk);
    desk.fixture.advance(Duration::from_millis(500));
    assert_eq!(desk.fixture.model().windows()[&window].place, before);
    assert_eq!(desk.fixture.model().overview(), Overview::Open);
    assert_eq!(
        desk.shell
            .drop_targets()
            .last()
            .map(|(_, space)| space.clone()),
        Some(None)
    );
}

/// An empty rectangle is a protocol error; a target of an output that was cleared is gone.
#[test]
fn an_empty_target_is_refused_and_a_cleared_one_is_not_hit() {
    let mut desk = desk();
    target(&mut desk);
    desk.shell.space_target_clear(&desk.outputs[0].object);
    desk.fixture.pump();
    open(&mut desk);
    let window = desk.windows[0];
    let at = centre_of(&desk, window);
    move_to(&mut desk, at);
    press(&mut desk);
    move_to(&mut desk, LogicalPoint { x: 50, y: 50 });
    release(&mut desk);
    assert!(desk.shell.drop_targets().is_empty(), "no target to hit");
    assert_ne!(
        desk.fixture.model().windows()[&window].place,
        SpacePlace::On(desk.second.clone())
    );
    desk.shell.space_target(
        &desk.outputs[0].object,
        desk.second.as_str(),
        (0, 0),
        (0, 10),
    );
    desk.fixture.pump();
    assert!(
        desk.fixture
            .client(desk.shell_id)
            .protocol_error()
            .is_some()
    );
}

/// A picture of the shown Space: a frame of the size asked for, drawn on the first request,
/// held while nothing changes and sent again when a window of the Space commits.
#[test]
fn a_space_picture_is_sent_on_damage_only() {
    let mut desk = desk();
    let output = output_key_of_first(&desk);
    let shown = desk
        .fixture
        .model()
        .spaces()
        .shown_on(&output)
        .expect("a shown Space")
        .id
        .clone();
    let mut capture =
        OutputCapture::start(desk.fixture.client(desk.shell_id).globals()).expect("the managers");
    settle_capture(&mut desk.fixture, &mut capture);
    capture
        .open_space(
            &desk.shell.object,
            "HEADLESS-1",
            shown.as_str(),
            Size::new(160, 120),
        )
        .expect("a session on the Space");
    settle_capture(&mut desk.fixture, &mut capture);
    assert_eq!(
        capture.session().expect("a session").size,
        Some(Size::new(160, 120))
    );
    capture.request(BufferAsk::Announced).expect("a frame");
    settle_capture(&mut desk.fixture, &mut capture);
    let frame = capture
        .take()
        .expect("answered")
        .expect("a frame, not a failure");
    assert_eq!((frame.size.w, frame.size.h), (160, 120));
    let rgba = frame.to_rgba8().expect("argb8888 reads as rgba");
    let opaque = rgba.chunks(4).filter(|pixel| pixel[3] == 255).count();
    assert!(opaque > 0, "the windows are in it");
    assert!(opaque < 160 * 120, "the background is transparent");

    capture.request(BufferAsk::Announced).expect("another");
    settle_capture(&mut desk.fixture, &mut capture);
    assert!(
        capture.take().is_none(),
        "nothing changed: the request waits"
    );
    let buffer = shm_buffer(
        desk.fixture.client(desk.app),
        PhysicalSize { w: 320, h: 200 },
        Pattern::Gradient,
    );
    desk.tops[0].show(&buffer);
    settle_capture(&mut desk.fixture, &mut capture);
    assert!(
        capture.take().is_some_and(|frame| frame.is_ok()),
        "a commit sends the held frame"
    );
}

/// A picture of a Space that has no window is made too, and is empty.
#[test]
fn a_picture_of_an_empty_space_is_made_and_transparent() {
    let mut desk = desk();
    let mut capture =
        OutputCapture::start(desk.fixture.client(desk.shell_id).globals()).expect("the managers");
    settle_capture(&mut desk.fixture, &mut capture);
    capture
        .open_space(
            &desk.shell.object,
            "HEADLESS-1",
            desk.second.as_str(),
            Size::new(80, 60),
        )
        .expect("a session");
    settle_capture(&mut desk.fixture, &mut capture);
    assert!(!capture.is_stopped(), "a Space that is not shown has one");
    capture.request(BufferAsk::Announced).expect("a frame");
    settle_capture(&mut desk.fixture, &mut capture);
    let frame = capture
        .take()
        .expect("answered")
        .expect("a frame, not a failure");
    let rgba = frame.to_rgba8().expect("argb8888 reads as rgba");
    assert!(rgba.chunks(4).all(|pixel| pixel[3] == 0), "nothing in it");
}

/// A picture of a Space that is not on the output asked for stops.
#[test]
fn a_picture_of_a_space_that_is_not_on_the_output_stops() {
    let mut desk = desk();
    let mut capture =
        OutputCapture::start(desk.fixture.client(desk.shell_id).globals()).expect("the managers");
    settle_capture(&mut desk.fixture, &mut capture);
    capture
        .open_space(
            &desk.shell.object,
            "HEADLESS-1",
            "s-nowhere",
            Size::new(80, 60),
        )
        .expect("a session");
    settle_capture(&mut desk.fixture, &mut capture);
    assert!(capture.is_stopped());
}

fn output_key_of_first(desk: &Desk) -> OutputKey {
    output_key(&desk.fixture)
}
