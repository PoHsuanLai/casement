//! Which input serial an explicit popup grab may answer (xdg_popup.grab: "the serial of the
//! input event that triggered the request"), and what the pointer is told when a popup goes
//! (P0-popups, W8). Qt opens a context menu on the release and grabs with the release's serial.

use super::xdg_shell::{click, map_toplevel, pointer, settle, windows};
use casement_core::DeviceId;
use casement_core::input::ButtonCode;
use casement_core::policy::ConnectionRole;
use casement_protocols::vocab::PressEdge;
use casement_testclient::seat::keyboard::Keyboard;
use casement_testclient::surface::popup::{Popup, PopupSpec};
use casement_testclient::surface::toplevel::{BoundPointer, Toplevel, WmBase, bind_pointer};
use casement_testclient::{Pattern, shm_buffer};
use casement_testkit::{ClientId, Fixture, FixtureSpec};
use smithay::reexports::wayland_server::Resource;
use wayland_client::Proxy;

const LEFT: ButtonCode = ButtonCode(0x110);

/// The protocol id of the surface the pointer is on (one client here, so ids are unambiguous).
fn focus_id(fixture: &Fixture) -> Option<u32> {
    fixture
        .pointer_focus()
        .map(|surface| surface.id().protocol_id())
}

/// A mapped window of one client that has been clicked twice (the first click only activates).
struct Desk {
    fixture: Fixture,
    id: ClientId,
    base: WmBase,
    parent: Toplevel,
    seat: BoundPointer,
    mouse: DeviceId,
}

fn desk() -> Desk {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let id = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let base = WmBase::bind(fixture.client(id), 7);
    let parent = map_toplevel(&mut fixture, id, &base, (400, 300));
    let mouse = pointer(&mut fixture);
    let seat = bind_pointer(fixture.client(id));
    fixture.pump();
    settle(&mut fixture);
    let window = windows(&fixture)[0];
    click(&mut fixture, mouse, window);
    click(&mut fixture, mouse, window);
    fixture.pump();
    Desk {
        fixture,
        id,
        base,
        parent,
        seat,
        mouse,
    }
}

impl Desk {
    /// Opens a popup that grabs with `serial`; whether it was dismissed at once.
    fn grab_with(&mut self, serial: u32) -> bool {
        let popup = self.popup_grabbing(self.id, serial);
        let dismissed = popup.log.with(|seen| seen.done) > 0;
        popup.destroy();
        self.fixture.pump();
        dismissed
    }

    fn popup_grabbing(&mut self, id: ClientId, serial: u32) -> Popup {
        let surface = self.fixture.client(id).create_surface().expect("a surface");
        let popup = Popup::of(
            self.fixture.client(id),
            &self.base,
            &self.parent,
            surface,
            PopupSpec::menu_at((20, 20)),
        );
        popup.grab(&self.seat.seat, serial);
        popup.surface.commit();
        self.fixture.pump();
        popup
    }

    fn click_again(&mut self) {
        self.fixture
            .input()
            .button(self.mouse, LEFT, PressEdge::Press);
        self.fixture
            .input()
            .button(self.mouse, LEFT, PressEdge::Release);
        self.fixture.pump();
    }
}

/// A menu opened on the release grabs with the release's serial (Qt), one opened on the press
/// with the press's (GTK), also when the release has already been sent.
#[test]
fn a_grab_may_answer_a_button_release_or_the_press_it_ended() {
    let mut desk = desk();
    let release = desk.seat.last_button().expect("a release was delivered");
    let press = desk.seat.last_press().expect("a press was delivered");
    assert_ne!(release, press);
    assert!(!desk.grab_with(release), "the release's serial answers");
    assert!(!desk.grab_with(press), "so does the press it ended");
}

/// A serial older than the client's latest input, one nobody was given and a serial that
/// another client was given are all refused.
#[test]
fn a_stale_forged_or_foreign_serial_does_not_grab() {
    let mut desk = desk();
    let stale = desk.seat.last_press().expect("a press");
    desk.click_again();
    assert!(desk.grab_with(stale), "a serial two inputs old is stale");
    let latest = desk.seat.last_button().expect("a release");
    assert!(desk.grab_with(latest.wrapping_add(1000)), "forged");
    // Another client holds a surface of its own, is given its own serials and cannot use ours.
    let other = desk.fixture.connect(ConnectionRole::Public);
    desk.fixture.pump();
    let other_base = WmBase::bind(desk.fixture.client(other), 7);
    let _other_window = map_toplevel(&mut desk.fixture, other, &other_base, (200, 200));
    let surface = desk
        .fixture
        .client(other)
        .create_surface()
        .expect("a surface");
    let popup = Popup::of(
        desk.fixture.client(other),
        &other_base,
        &_other_window,
        surface,
        PopupSpec::menu_at((5, 5)),
    );
    let other_seat = bind_pointer(desk.fixture.client(other));
    desk.fixture.pump();
    popup.grab(&other_seat.seat, latest);
    popup.surface.commit();
    desk.fixture.pump();
    assert!(popup.log.with(|seen| seen.done) > 0, "another client's");
}

/// A grabbed popup is dismissed when its client maps a new toplevel (xdg_popup.grab: a popup
/// of the grabbing client is dismissed when the keyboard focus moves to another toplevel).
#[test]
fn a_grabbed_popup_is_done_when_the_client_maps_another_toplevel() {
    let mut desk = desk();
    let serial = desk.seat.last_button().expect("a release");
    let popup = desk.popup_grabbing(desk.id, serial);
    assert_eq!(popup.log.with(|seen| seen.done), 0);
    let _second = map_toplevel(&mut desk.fixture, desk.id, &desk.base, (200, 200));
    desk.fixture.pump();
    let second = *windows(&desk.fixture).last().expect("the second window");
    desk.fixture.activate_window(second);
    assert_eq!(popup.log.with(|seen| seen.done), 1);
}

/// A press on the window a grabbed popup hangs from, outside the popup, dismisses it, though the
/// window is the popup's own client; the press does not reach the window.
#[test]
fn a_press_on_the_popup_s_own_window_dismisses_it() {
    let mut desk = desk();
    let serial = desk.seat.last_button().expect("a release");
    let popup = desk.popup_grabbing(desk.id, serial);
    assert_eq!(popup.log.with(|seen| seen.done), 0);
    let before = desk.seat.last_press();
    desk.click_again();
    assert_eq!(
        popup.log.with(|seen| seen.done),
        1,
        "the popup was dismissed"
    );
    assert_eq!(
        desk.seat.last_press(),
        before,
        "the window was not given the press"
    );
}

/// A submenu that grabs with the serial its parent grabbed with (a timer opens it, with no new
/// input) keeps the grab: the keyboard stays with the submenu, not the window below.
#[test]
fn a_nested_grab_with_the_same_serial_keeps_the_keyboard() {
    let mut desk = desk();
    let _keyboard = Keyboard::bind(desk.fixture.client(desk.id)).expect("a keyboard");
    desk.fixture.pump();
    let serial = desk.seat.last_button().expect("a release");
    let menu = desk.popup_grabbing(desk.id, serial);
    menu.ack_latest().expect("configured");
    let buffer = shm_buffer(
        desk.fixture.client(desk.id),
        casement_core::geometry::PhysicalSize { w: 100, h: 50 },
        Pattern::Solid(0xff40_4040),
    );
    menu.show(&buffer);
    desk.fixture.pump();
    let surface = desk
        .fixture
        .client(desk.id)
        .create_surface()
        .expect("a surface");
    let sub = Popup::of_popup(
        desk.fixture.client(desk.id),
        &desk.base,
        &menu,
        surface,
        PopupSpec::menu_at((5, 5)),
    );
    sub.grab(&desk.seat.seat, serial);
    sub.surface.commit();
    desk.fixture.pump();
    assert_eq!(sub.log.with(|seen| seen.done), 0);
    assert_eq!(menu.log.with(|seen| seen.done), 0);
    let entered: Vec<String> = desk
        .fixture
        .client(desk.id)
        .transcript()
        .0
        .iter()
        .filter(|line| line.object.starts_with("wl_keyboard#") && line.event == "enter")
        .map(|line| line.args.join(","))
        .collect();
    let last = entered.last().expect("the keyboard entered something");
    assert!(
        last.contains(&format!("wl_surface#{}", sub.surface.id().protocol_id())),
        "the keyboard's last enter is the submenu: {entered:?}"
    );
}

/// When the popup under the pointer goes, the pointer leaves it and enters what is under it
/// then, before the popup's surface is destroyed: a client that never hears of the leave
/// is told of an enter without one.
#[test]
fn the_pointer_leaves_a_popup_that_is_destroyed_under_it() {
    let mut desk = desk();
    let serial = desk.seat.last_button().expect("a release");
    let popup = desk.popup_grabbing(desk.id, serial);
    popup.ack_latest().expect("configured");
    let buffer = shm_buffer(
        desk.fixture.client(desk.id),
        casement_core::geometry::PhysicalSize { w: 100, h: 50 },
        Pattern::Solid(0xff40_4040),
    );
    popup.show(&buffer);
    desk.fixture.pump();
    let window = windows(&desk.fixture)[0];
    let origin = super::xdg_shell::rect_of(&desk.fixture, window).origin;
    desk.fixture.input().pointer_to(
        desk.mouse,
        casement_core::geometry::LogicalPoint {
            x: origin.x + 30,
            y: origin.y + 30,
        },
    );
    desk.fixture.pump();
    let popup_name = format!("wl_surface#{}", popup.surface.id().protocol_id());
    let before = desk.fixture.client(desk.id).transcript().0.len();
    popup.destroy();
    desk.fixture.pump();
    let lines: Vec<_> = desk.fixture.client(desk.id).transcript().0[before..]
        .iter()
        .filter(|line| line.object.starts_with("wl_pointer#"))
        .map(|line| (line.event.clone(), line.args.clone()))
        .collect();
    assert!(
        lines
            .iter()
            .any(|(event, args)| event == "leave" && args.iter().any(|a| a.contains(&popup_name))),
        "the popup's client was told the pointer left it: {lines:?}"
    );
    assert!(
        lines.iter().any(|(event, _)| event == "enter"),
        "and that it entered the parent: {lines:?}"
    );
}

/// A popup that asked to follow its parent is placed again when the parent moves, and a popup that
/// did not stays where it was configured.
#[test]
fn a_reactive_popup_follows_its_parent_and_another_does_not() {
    let mut desk = desk();
    let place = |desk: &mut Desk, spec: PopupSpec, reactive: bool| {
        let surface = desk
            .fixture
            .client(desk.id)
            .create_surface()
            .expect("a surface");
        let client = desk.fixture.client(desk.id);
        let popup = match reactive {
            true => Popup::reactive_of(client, &desk.base, &desk.parent, surface, spec),
            false => Popup::of(client, &desk.base, &desk.parent, surface, spec),
        };
        popup.surface.commit();
        desk.fixture.pump();
        popup.ack_latest().expect("configured");
        let buffer = shm_buffer(
            desk.fixture.client(desk.id),
            casement_core::geometry::PhysicalSize { w: 100, h: 50 },
            Pattern::Solid(0xff40_4040),
        );
        popup.show(&buffer);
        desk.fixture.pump();
        popup
    };
    let spec = PopupSpec {
        anchor_rect: (0, 0, 1, 1),
        gravity: casement_testclient::surface::toplevel::shell::xdg_positioner::Gravity::TopLeft,
        ..PopupSpec::menu_at((0, 0))
    };
    let follows = place(&mut desk, spec, true);
    let stays = place(&mut desk, spec, false);
    let (before_f, before_s) = (follows.geometry(), stays.geometry());
    let window = windows(&desk.fixture)[0];
    // Far to the left: the popup's room, the output, now cuts it where it hung before.
    desk.fixture
        .place_window(window, casement_core::geometry::LogicalPoint { x: 5, y: 5 });
    assert_ne!(
        follows.geometry(),
        before_f,
        "the reactive popup was configured again"
    );
    assert_eq!(stays.geometry(), before_s, "the other one was not");
}

/// The new position of a repositioned popup takes effect when the client acks the configure,
/// with no commit after it (xdg_popup.repositioned).
#[test]
fn a_repositioned_popup_is_where_the_ack_put_it_without_a_commit() {
    let mut desk = desk();
    let surface = desk
        .fixture
        .client(desk.id)
        .create_surface()
        .expect("a surface");
    let popup = Popup::of(
        desk.fixture.client(desk.id),
        &desk.base,
        &desk.parent,
        surface,
        PopupSpec::menu_at((20, 20)),
    );
    popup.surface.commit();
    desk.fixture.pump();
    popup.ack_latest().expect("configured");
    let buffer = shm_buffer(
        desk.fixture.client(desk.id),
        casement_core::geometry::PhysicalSize { w: 100, h: 50 },
        Pattern::Solid(0xff40_4040),
    );
    popup.show(&buffer);
    desk.fixture.pump();
    let window = windows(&desk.fixture)[0];
    let origin = super::xdg_shell::rect_of(&desk.fixture, window).origin;
    let at = |x: i32, y: i32| casement_core::geometry::LogicalPoint {
        x: origin.x + x,
        y: origin.y + y,
    };
    desk.fixture.input().pointer_to(desk.mouse, at(60, 40));
    desk.fixture.pump();
    assert_eq!(
        focus_id(&desk.fixture),
        Some(popup.surface.id().protocol_id()),
        "the popup is under the pointer where it was placed"
    );
    popup.reposition(
        desk.fixture.client(desk.id),
        &desk.base,
        PopupSpec::menu_at((200, 200)),
        3,
    );
    desk.fixture.pump();
    popup.ack_latest().expect("the reposition's configure");
    desk.fixture.pump();
    desk.fixture.input().pointer_to(desk.mouse, at(60, 41));
    desk.fixture.pump();
    assert_ne!(
        focus_id(&desk.fixture),
        Some(popup.surface.id().protocol_id()),
        "the popup left its old place"
    );
    desk.fixture.input().pointer_to(desk.mouse, at(250, 240));
    desk.fixture.pump();
    assert_eq!(
        focus_id(&desk.fixture),
        Some(popup.surface.id().protocol_id()),
        "and is at its new one"
    );
}

/// A window that is being moved by the person loses the pointer's focus (xdg_toplevel.move: "the
/// surface will lose the focus of the device used for the move") and has it back when the
/// button is let go.
#[test]
fn a_window_in_an_interactive_move_loses_the_pointer_and_gets_it_back() {
    let mut desk = desk();
    let window = windows(&desk.fixture)[0];
    let origin = super::xdg_shell::rect_of(&desk.fixture, window).origin;
    desk.fixture.input().pointer_to(
        desk.mouse,
        casement_core::geometry::LogicalPoint {
            x: origin.x + 50,
            y: origin.y + 50,
        },
    );
    desk.fixture
        .input()
        .button(desk.mouse, LEFT, PressEdge::Press);
    desk.fixture.pump();
    let serial = desk.seat.last_press().expect("a press");
    assert!(
        desk.fixture.pointer_focus().is_some(),
        "the window has the pointer"
    );
    desk.parent.request_move(&desk.seat.seat, serial);
    desk.fixture.pump();
    assert!(
        desk.fixture.pointer_focus().is_none(),
        "the pointer left the window being moved"
    );
    desk.fixture.input().pointer_to(
        desk.mouse,
        casement_core::geometry::LogicalPoint {
            x: origin.x + 80,
            y: origin.y + 80,
        },
    );
    desk.fixture.pump();
    assert!(
        desk.fixture.pointer_focus().is_none(),
        "and stays out while it moves"
    );
    desk.fixture
        .input()
        .button(desk.mouse, LEFT, PressEdge::Release);
    desk.fixture.advance(std::time::Duration::from_secs(1));
    desk.fixture.pump();
    assert!(
        desk.fixture.pointer_focus().is_some(),
        "it has the pointer again"
    );
}
