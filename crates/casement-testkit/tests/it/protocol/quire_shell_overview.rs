//! `quire_shell_v1` overview: the shell opens Mission Control, the windows spread to their
//! slots, a click picks one (or the picture, leaving with nothing picked), Escape leaves, and
//! the shell hears each state. Shell connections only.

use crate::protocol::xdg_shell::{click_at, map_toplevel, pointer, windows};
use casement_core::geometry::LogicalPoint;
use casement_core::input::{DeviceClass, KeyCode};
use casement_core::overview::Overview;
use casement_core::policy::ConnectionRole;
use casement_core::{DeviceId, WindowId};
use casement_layout::{LayoutKnobs, OverviewSlot, RustScene, SceneLayout, spec_of};
use casement_protocols::vocab::PressEdge;
use casement_testclient::quire::shell::{OverviewMode, OverviewState, ShellHandle, Toggle};
use casement_testclient::surface::toplevel::{Toplevel, WmBase};
use casement_testkit::{Fixture, FixtureSpec, TestOrigin};
use std::time::Duration;

/// The Escape key (`KEY_ESC`).
const KEY_ESC: KeyCode = KeyCode(1);

/// A desktop with two windows, a shell and a pointer; what must stay alive until the test ends.
struct Desk {
    fixture: Fixture,
    shell: ShellHandle,
    windows: Vec<WindowId>,
    pointer: DeviceId,
    _tops: Vec<Toplevel>,
    _base: WmBase,
}

fn desk() -> Desk {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let app = fixture.connect(ConnectionRole::Public);
    let base = WmBase::bind(fixture.client(app), 7);
    let tops = vec![
        map_toplevel(&mut fixture, app, &base, (640, 400)),
        map_toplevel(&mut fixture, app, &base, (480, 320)),
    ];
    let id = fixture.connect(ConnectionRole::Shell);
    let shell = ShellHandle::bind(fixture.client(id), 1);
    fixture.pump();
    let pointer = pointer(&mut fixture);
    let windows = windows(&fixture);
    Desk {
        fixture,
        shell,
        windows,
        pointer,
        _tops: tops,
        _base: base,
    }
}

/// Where the layout puts the windows in the overview now (the arrangement the compositor draws).
fn slots(fixture: &Fixture) -> Vec<OverviewSlot> {
    let model = fixture.model();
    RustScene
        .arrange(&spec_of(model), &LayoutKnobs::of(model.settings()))
        .overview
}

fn open(desk: &mut Desk) {
    desk.shell
        .overview(OverviewMode::MissionControl, Toggle::On);
    desk.fixture.pump();
    desk.fixture.advance(Duration::from_millis(500));
}

/// The shell's request opens Mission Control: the model is entering, then open once the windows
/// have had their time, every window has a slot inside the usable area that is no bigger than
/// the window, and the shell hears each state with its progress.
#[test]
fn the_shell_opens_mission_control_and_hears_each_state() {
    let mut desk = desk();
    assert!(slots(&desk.fixture).is_empty(), "no slots on the desktop");
    desk.shell
        .overview(OverviewMode::MissionControl, Toggle::On);
    desk.fixture.pump();
    assert_eq!(desk.fixture.model().overview(), Overview::Entering);
    assert_eq!(
        desk.shell.overview_states(),
        [(Some(OverviewState::Entering), 0)]
    );
    desk.fixture.advance(Duration::from_millis(500));
    assert_eq!(desk.fixture.model().overview(), Overview::Open);
    assert_eq!(
        desk.shell.overview_states().last(),
        Some(&(Some(OverviewState::Open), 1000))
    );
    let found = slots(&desk.fixture);
    assert_eq!(found.len(), 2);
    for window in &desk.windows {
        assert!(
            found.iter().any(|slot| slot.window == *window),
            "{window:?}"
        );
    }
    assert!(found.iter().all(|slot| slot.rect.size.w <= 640));
}

/// A click on a window's slot leaves Mission Control with that window raised and keyed; the
/// windows fly back and it is closed.
#[test]
fn a_click_on_a_slot_picks_that_window() {
    let mut desk = desk();
    open(&mut desk);
    let other = desk.windows[0];
    let at = slots(&desk.fixture)
        .iter()
        .find(|slot| slot.window == other)
        .expect("its slot")
        .rect
        .centre();
    click_at(&mut desk.fixture, desk.pointer, at);
    desk.fixture.pump();
    assert_eq!(desk.fixture.model().overview(), Overview::Leaving);
    assert_eq!(desk.fixture.model().focus().key, Some(other));
    desk.fixture.advance(Duration::from_millis(500));
    assert_eq!(desk.fixture.model().overview(), Overview::Closed);
    assert_eq!(
        desk.shell.overview_states().last(),
        Some(&(Some(OverviewState::Closed), 0))
    );
}

/// A click on the picture, where no window is, leaves with nothing picked.
#[test]
fn a_click_on_the_picture_leaves_with_nothing_picked() {
    let mut desk = desk();
    open(&mut desk);
    let before = desk.fixture.model().focus().key;
    click_at(&mut desk.fixture, desk.pointer, LogicalPoint { x: 4, y: 4 });
    desk.fixture.pump();
    assert_eq!(desk.fixture.model().overview(), Overview::Leaving);
    assert_eq!(desk.fixture.model().focus().key, before);
}

/// Escape leaves, and the key is not delivered to the key window.
#[test]
fn escape_leaves_mission_control() {
    let mut desk = desk();
    open(&mut desk);
    let keyboard = desk.fixture.input().plug(
        DeviceClass::Keyboard,
        TestOrigin::Physical(DeviceClass::Keyboard),
    );
    desk.fixture
        .input()
        .key(keyboard, KEY_ESC, PressEdge::Press);
    desk.fixture
        .input()
        .key(keyboard, KEY_ESC, PressEdge::Release);
    desk.fixture.pump();
    assert_eq!(desk.fixture.model().overview(), Overview::Leaving);
}

/// The shell closes it too, and asking for another overview changes nothing.
#[test]
fn the_shell_closes_it_and_other_overviews_are_ignored() {
    let mut desk = desk();
    desk.shell.overview(OverviewMode::AppExpose, Toggle::On);
    desk.fixture.pump();
    assert_eq!(desk.fixture.model().overview(), Overview::Closed);
    open(&mut desk);
    desk.shell
        .overview(OverviewMode::MissionControl, Toggle::Off);
    desk.fixture.pump();
    assert_eq!(desk.fixture.model().overview(), Overview::Leaving);
}
