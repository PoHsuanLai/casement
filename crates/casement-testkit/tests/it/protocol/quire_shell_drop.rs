//! `quire_shell_v1` version 3: the pictures of Spaces that are not shown, the new-Space tile
//! (`space_target_new`, `drop_target_new`) and a window dragged to another output in Mission
//! Control. Shell connections only. The desk and its helpers are `quire_shell_spaces.rs`'s.

use super::quire_shell_spaces::{Desk, centre_of, desk, move_to, open, press, release, target};
use crate::protocol::image_capture::settle as settle_capture;
use crate::protocol::xdg_shell::{map_toplevel, pointer, windows};
use casement_core::geometry::LogicalPoint;
use casement_core::overview::Overview;
use casement_core::policy::ConnectionRole;
use casement_core::window::SpacePlace;
use casement_core::{OutputKey, WindowId};
use casement_testclient::capture::{BufferAsk, OutputCapture, Size};
use casement_testclient::quire::shell::{BoundOutput, ShellHandle, Shown};
use casement_testclient::surface::toplevel::WmBase;
use casement_testclient::{Pattern, shm_buffer};
use casement_testkit::{Fixture, FixtureSpec};
use porter_core::SpaceId;
use std::time::Duration;

/// The tile in the top left corner, where no slot is (slots start below the 112 px strip).
const TILE_AT: (i32, i32) = (10, 10);
const TILE_SIZE: (u32, u32) = (100, 90);

fn tile(desk: &mut Desk) {
    let output = &desk.outputs[0].object;
    desk.shell.space_target_new(output, TILE_AT, TILE_SIZE);
    desk.fixture.pump();
}

/// Window 0 is pressed on its slot and carried to `to`; the button is still down.
fn carry(desk: &mut Desk, to: LogicalPoint) -> WindowId {
    let window = desk.windows[0];
    let at = centre_of(desk, window);
    move_to(desk, at);
    press(desk);
    move_to(desk, to);
    window
}

fn spaces_of_first_output(desk: &Desk) -> Vec<SpaceId> {
    let output = desk
        .fixture
        .model()
        .outputs()
        .0
        .keys()
        .next()
        .cloned()
        .expect("an output");
    desk.fixture
        .model()
        .spaces()
        .row(&output)
        .into_iter()
        .map(|space| space.id.clone())
        .collect()
}

/// The windows of a Space the output does not show are drawn in its picture, and a commit of one
/// of them (not of a window of another Space) sends a held frame.
#[test]
fn a_picture_of_a_hidden_space_follows_its_own_windows() {
    let mut desk = desk();
    target(&mut desk);
    open(&mut desk);
    let window = carry(&mut desk, LogicalPoint { x: 50, y: 50 });
    release(&mut desk);
    assert_eq!(
        desk.fixture.model().windows()[&window].place,
        SpacePlace::On(desk.second.clone()),
        "window 0 now lives on the hidden Space"
    );
    let mut capture =
        OutputCapture::start(desk.fixture.client(desk.shell_id).globals()).expect("the managers");
    settle_capture(&mut desk.fixture, &mut capture);
    capture
        .open_space(
            &desk.shell.object,
            "HEADLESS-1",
            desk.second.as_str(),
            Size::new(160, 120),
        )
        .expect("a session on the hidden Space");
    settle_capture(&mut desk.fixture, &mut capture);
    assert!(!capture.is_stopped(), "a hidden Space has a picture");
    capture.request(BufferAsk::Announced).expect("a frame");
    settle_capture(&mut desk.fixture, &mut capture);
    let frame = capture
        .take()
        .expect("answered")
        .expect("a frame, not a failure");
    let rgba = frame.to_rgba8().expect("argb8888 reads as rgba");
    let opaque = rgba.chunks(4).filter(|pixel| pixel[3] == 255).count();
    assert!(opaque > 0, "its window is in it");
    assert!(
        opaque < 160 * 120,
        "the wallpaper is the shell's: transparent"
    );

    capture.request(BufferAsk::Announced).expect("another");
    settle_capture(&mut desk.fixture, &mut capture);
    assert!(capture.take().is_none(), "nothing changed: held");
    // The other window sits on the shown Space: its commit is not news to this picture.
    let buffer = shm_buffer(
        desk.fixture.client(desk.app),
        casement_core::geometry::PhysicalSize { w: 240, h: 160 },
        Pattern::Gradient,
    );
    desk.tops[1].show(&buffer);
    settle_capture(&mut desk.fixture, &mut capture);
    assert!(capture.take().is_none(), "a window of another Space");
    let buffer = shm_buffer(
        desk.fixture.client(desk.app),
        casement_core::geometry::PhysicalSize { w: 320, h: 200 },
        Pattern::Gradient,
    );
    desk.tops[0].show(&buffer);
    settle_capture(&mut desk.fixture, &mut capture);
    assert!(
        capture.take().is_some_and(|frame| frame.is_ok()),
        "a commit of its own window sends the held frame"
    );
}

/// A window dropped on the new-Space tile makes a Space with it, at the end of the row; the
/// shell hears the tile lit while the window is over it and unlit when the drag ends, and
/// Mission Control stays open.
#[test]
fn a_window_dropped_on_the_new_space_tile_gets_a_space_of_its_own() {
    let mut desk = desk();
    tile(&mut desk);
    open(&mut desk);
    let before = spaces_of_first_output(&desk);
    let output = desk.outputs[0].object.clone();
    let window = carry(&mut desk, LogicalPoint { x: 50, y: 50 });
    assert!(
        desk.shell
            .drop_targets_new()
            .contains(&(output.clone(), Some(Shown::Shown))),
        "the tile is lit"
    );
    release(&mut desk);
    let after = spaces_of_first_output(&desk);
    assert_eq!(after.len(), before.len() + 1, "a Space was made");
    assert_eq!(
        desk.fixture.model().windows()[&window].place,
        SpacePlace::On(after.last().expect("a last Space").clone())
    );
    assert_eq!(desk.fixture.model().overview(), Overview::Open);
    assert_eq!(
        desk.shell.drop_targets_new().last().cloned(),
        Some((output, Some(Shown::Hidden))),
        "the drag's end unlights it"
    );
}

/// The tile is in front of a Space target under it, and leaving it unlights it.
#[test]
fn the_tile_wins_over_a_space_target_and_leaving_it_unlights_it() {
    let mut desk = desk();
    target(&mut desk);
    tile(&mut desk);
    open(&mut desk);
    let before = spaces_of_first_output(&desk);
    let output = desk.outputs[0].object.clone();
    let window = carry(&mut desk, LogicalPoint { x: 50, y: 50 });
    assert!(
        !desk
            .shell
            .drop_targets()
            .contains(&(output.clone(), Some(desk.second.as_str().to_owned()))),
        "the Space target under the tile is not lit"
    );
    move_to(&mut desk, LogicalPoint { x: 560, y: 440 });
    assert_eq!(
        desk.shell.drop_targets_new().last().cloned(),
        Some((output, Some(Shown::Hidden))),
        "leaving the tile unlights it"
    );
    release(&mut desk);
    desk.fixture.advance(Duration::from_millis(500));
    assert_eq!(spaces_of_first_output(&desk), before, "no Space was made");
    assert_ne!(
        desk.fixture.model().windows()[&window].place,
        SpacePlace::On(desk.second.clone())
    );
    assert_eq!(desk.fixture.model().overview(), Overview::Open);
}

/// An empty rectangle removes the tile (no error); `space_target_clear` leaves it.
#[test]
fn an_empty_rectangle_removes_the_tile_and_clearing_the_spaces_keeps_it() {
    let mut desk = desk();
    tile(&mut desk);
    desk.shell.space_target_clear(&desk.outputs[0].object);
    desk.fixture.pump();
    open(&mut desk);
    let before = spaces_of_first_output(&desk);
    let window = carry(&mut desk, LogicalPoint { x: 50, y: 50 });
    release(&mut desk);
    assert_eq!(
        spaces_of_first_output(&desk).len(),
        before.len() + 1,
        "the tile survived a clear"
    );
    assert!(matches!(
        desk.fixture.model().windows()[&window].place,
        SpacePlace::On(_)
    ));

    // Remove it with an empty rectangle: a second drag finds nothing.
    desk.shell
        .space_target_new(&desk.outputs[0].object, (0, 0), (0, 0));
    desk.fixture.pump();
    assert!(
        desk.fixture
            .client(desk.shell_id)
            .protocol_error()
            .is_none(),
        "an empty rectangle is not an error here"
    );
    let heard = desk.shell.drop_targets_new().len();
    let other = desk.windows[1];
    let at = centre_of(&desk, other);
    move_to(&mut desk, at);
    press(&mut desk);
    move_to(&mut desk, LogicalPoint { x: 50, y: 50 });
    release(&mut desk);
    assert_eq!(desk.shell.drop_targets_new().len(), heard, "no tile to hit");
}

/// A shell that bound version 2 hears none of the version 3 events.
#[test]
fn a_version_two_shell_does_not_hear_the_tile() {
    let mut desk = desk();
    tile(&mut desk);
    let old_id = desk.fixture.connect(ConnectionRole::Shell);
    let old = ShellHandle::bind(desk.fixture.client(old_id), 2);
    desk.fixture.pump();
    open(&mut desk);
    carry(&mut desk, LogicalPoint { x: 50, y: 50 });
    release(&mut desk);
    assert!(!desk.shell.drop_targets_new().is_empty());
    assert!(old.drop_targets_new().is_empty());
}

/// Two outputs, two windows on the left one, a version 3 shell with its outputs bound.
struct Pair {
    fixture: Fixture,
    shell: ShellHandle,
    outputs: Vec<BoundOutput>,
    windows: Vec<WindowId>,
    pointer: casement_core::DeviceId,
    _base: WmBase,
}

fn pair() -> Pair {
    let spec = FixtureSpec::desktop().with_outputs(&[
        "HEADLESS-1:640x480@60000:120:0,0",
        "HEADLESS-2:640x480@60000:120:640,0",
    ]);
    let mut fixture = Fixture::start(spec).expect("fixture");
    let app = fixture.connect(ConnectionRole::Public);
    let base = WmBase::bind(fixture.client(app), 7);
    map_toplevel(&mut fixture, app, &base, (320, 200));
    map_toplevel(&mut fixture, app, &base, (240, 160));
    let shell_id = fixture.connect(ConnectionRole::Shell);
    let shell = ShellHandle::bind(fixture.client(shell_id), 3);
    let outputs = ShellHandle::bind_outputs(fixture.client(shell_id));
    let pointer = pointer(&mut fixture);
    let windows = windows(&fixture);
    fixture.pump();
    Pair {
        fixture,
        shell,
        outputs,
        windows,
        pointer,
        _base: base,
    }
}

fn connector(pair: &Pair, name: &str) -> BoundOutput {
    pair.outputs
        .iter()
        .find(|bound| bound.connector().as_deref() == Some(name))
        .cloned()
        .expect("a bound output")
}

fn key_of(pair: &Pair, name: &str) -> OutputKey {
    pair.fixture
        .model()
        .outputs()
        .0
        .iter()
        .find(|(_, plugged)| plugged.facts.name.as_str() == name)
        .map(|(key, _)| key.clone())
        .expect("an output")
}

fn first_space(pair: &Pair, output: &OutputKey) -> SpaceId {
    pair.fixture.model().spaces().row(output)[0].id.clone()
}

fn drag_across(pair: &mut Pair, to: LogicalPoint) -> WindowId {
    let window = pair.windows[0];
    pair.shell.overview(
        casement_testclient::quire::shell::OverviewMode::MissionControl,
        casement_testclient::quire::shell::Toggle::On,
    );
    pair.fixture.pump();
    pair.fixture.advance(Duration::from_millis(500));
    let model = pair.fixture.model();
    let at = casement_layout::RustScene
        .arrange(
            &casement_layout::spec_of(model),
            &casement_layout::LayoutKnobs::of(model.settings()),
        )
        .overview
        .iter()
        .find(|slot| slot.window == window)
        .expect("its slot")
        .rect
        .centre();
    pair.fixture.input().pointer_to(pair.pointer, at);
    pair.fixture.pump();
    let left = casement_core::input::ButtonCode(0x110);
    pair.fixture
        .input()
        .button(pair.pointer, left, casement_core::input::Edge::Press);
    pair.fixture.pump();
    pair.fixture.input().pointer_to(pair.pointer, to);
    pair.fixture.pump();
    window
}

fn let_go(pair: &mut Pair) {
    let left = casement_core::input::ButtonCode(0x110);
    pair.fixture
        .input()
        .button(pair.pointer, left, casement_core::input::Edge::Release);
    pair.fixture.pump();
}

/// A window dragged onto the other output's Space target is dropped on that output's Space and
/// keeps its place about the output's corner.
#[test]
fn a_window_dragged_to_another_outputs_target_moves_to_its_space() {
    let mut pair = pair();
    let right = key_of(&pair, "HEADLESS-2");
    let right_space = first_space(&pair, &right);
    let bound = connector(&pair, "HEADLESS-2");
    pair.shell
        .space_target(&bound.object, right_space.as_str(), (700, 10), (100, 90));
    pair.fixture.pump();
    let window = pair.windows[0];
    let from = pair.fixture.model().windows()[&window].clone();
    let window = drag_across(&mut pair, LogicalPoint { x: 750, y: 50 });
    assert!(
        pair.shell
            .drop_targets()
            .contains(&(bound.object.clone(), Some(right_space.as_str().to_owned()))),
        "the other output's tile is lit"
    );
    let_go(&mut pair);
    let moved = pair.fixture.model().windows()[&window].clone();
    assert_eq!(moved.place, SpacePlace::On(right_space));
    assert_ne!(
        pair.fixture.model().output_of(&moved),
        pair.fixture.model().output_of(&from)
    );
    assert_eq!(pair.fixture.model().overview(), Overview::Open);
}

/// Released over the other output but off its targets, the window goes to the Space that output
/// shows, as a window dragged to another display does.
#[test]
fn a_window_released_over_another_output_goes_to_the_space_it_shows() {
    let mut pair = pair();
    let right = key_of(&pair, "HEADLESS-2");
    let shown = pair
        .fixture
        .model()
        .spaces()
        .shown_on(&right)
        .expect("a shown Space")
        .id
        .clone();
    let window = drag_across(&mut pair, LogicalPoint { x: 900, y: 300 });
    let_go(&mut pair);
    assert_eq!(
        pair.fixture.model().windows()[&window].place,
        SpacePlace::On(shown)
    );
    assert_eq!(pair.fixture.model().overview(), Overview::Open);
}

/// Released over its own output and off every target, the window flies back (nothing changes).
#[test]
fn a_window_released_over_its_own_output_flies_back() {
    let mut pair = pair();
    let window = pair.windows[0];
    let before = pair.fixture.model().windows()[&window].place.clone();
    drag_across(&mut pair, LogicalPoint { x: 560, y: 440 });
    let_go(&mut pair);
    assert_eq!(pair.fixture.model().windows()[&window].place, before);
}
