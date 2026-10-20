//! Agent leases (`LeaseWindow`): only the agent tier may lease, only a window that is there, and
//! what the lease's socket sends reaches the leased window and nothing else; once the lease
//! ends, or its window closes, it reaches nothing.

use crate::protocol::{map_toplevel, rect_of, settle, windows};
use casement_core::DeviceId;
use casement_core::geometry::{LogicalPoint, LogicalRect};
use casement_core::input::{ButtonCode, DeviceClass, Edge, InputEvent, InputKind, KeyCode, Origin};
use casement_core::policy::ConnectionRole;
use casement_core::{LeaseId, WindowId};
use casement_ipc::dbus::ConnectRefusal;
use casement_testclient::TestClient;
use casement_testclient::seat::keyboard::Keyboard;
use casement_testclient::seat::pointer::Pointer;
use casement_testclient::surface::layer::{
    Anchor, KeyboardInteractivity, Layer, LayerAsk, LayerSurface,
};
use casement_testclient::surface::toplevel::{Toplevel, WmBase};
use casement_testkit::fakes::callers;
use casement_testkit::{ClientId, Fixture, FixtureSpec, TestOrigin};
use casement_wayland::foreign_toplevel_list::identifier_of;
use std::time::Duration;

const BTN_LEFT: ButtonCode = ButtonCode(0x110);

/// The lease's device, numbered like a remote socket's.
const DEVICE: DeviceId = DeviceId(0x8000_0021);

/// How many events named `event` the objects of interface `object` received.
fn count(client: &TestClient, object: &str, event: &str) -> usize {
    client
        .transcript()
        .0
        .iter()
        .filter(|line| line.object.starts_with(object) && line.event == event)
        .count()
}

/// What a client keeps alive for its window to stay mapped.
type Kept = Vec<(Pointer, WmBase, Toplevel)>;

/// A desktop with two clients, each with a pointer and one mapped window: the first window is
/// under the second where they overlap.
fn two_windows() -> (Fixture, [(ClientId, WindowId); 2], Kept) {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let mut placed = Vec::new();
    let mut kept = Vec::new();
    for size in [(600, 400), (300, 200)] {
        let id = fixture.connect(ConnectionRole::Public);
        fixture.pump();
        let pointer = Pointer::bind(fixture.client(id)).expect("pointer");
        let base = WmBase::bind(fixture.client(id), 7);
        let before = windows(&fixture);
        let top = map_toplevel(&mut fixture, id, &base, size);
        let window = windows(&fixture)
            .into_iter()
            .find(|window| !before.contains(window))
            .expect("the new window");
        placed.push((id, window));
        kept.push((pointer, base, top));
    }
    settle(&mut fixture);
    (fixture, [placed[0], placed[1]], kept)
}

/// A point of `within`'s content (clear of its title bar and edges) that `outside` does not
/// cover.
fn uncovered(within: LogicalRect, outside: LogicalRect) -> LogicalPoint {
    let inset = 60;
    let (x0, y0) = (within.origin.x + inset, within.origin.y + inset);
    let (x1, y1) = (
        within.origin.x + within.size.w as i32 - inset,
        within.origin.y + within.size.h as i32 - inset,
    );
    [(x0, y1), (x1, y1), (x1, y0), (x0, y0)]
        .into_iter()
        .map(|(x, y)| LogicalPoint { x, y })
        .find(|point| !outside.contains(*point))
        .expect("the larger window shows past the smaller one")
}

/// One event of the lease, by the path the lease socket's input takes.
fn send(fixture: &mut Fixture, lease: u64, kind: InputKind) {
    let at = fixture.now();
    fixture.deliver_remote(InputEvent {
        origin: Origin::Agent(LeaseId(lease)),
        device: DEVICE,
        at,
        kind,
    });
}

/// A click of the lease at `at`.
fn click(fixture: &mut Fixture, lease: u64, at: LogicalPoint) {
    for kind in [
        InputKind::MotionTo { at },
        InputKind::Button {
            button: BTN_LEFT,
            edge: Edge::Press,
        },
        InputKind::Button {
            button: BTN_LEFT,
            edge: Edge::Release,
        },
    ] {
        send(fixture, lease, kind);
    }
}

#[test]
fn only_the_agent_tier_may_lease_and_only_a_window_that_is_there() {
    let (mut fixture, [(_, under), _], _kept) = two_windows();
    let window = identifier_of(under);
    for (case, caller) in [
        ("the shell", callers::shell()),
        ("the portal", callers::portal()),
        ("another desktop program", callers::companion()),
    ] {
        assert_eq!(
            fixture.lease_window(&caller, &window, 7).err(),
            Some(ConnectRefusal::RoleNotAllowed),
            "{case}"
        );
    }
    let agent = callers::agent_tier();
    for missing in ["w99999", "not-a-window", ""] {
        assert_eq!(
            fixture.lease_window(&agent, missing, 7).err(),
            Some(ConnectRefusal::NoSuchWindow),
            "{missing:?}"
        );
    }
    assert!(fixture.model().leases().iter().next().is_none());
    let (lease, _socket) = fixture.lease_window(&agent, &window, 7).expect("granted");
    assert_eq!(
        fixture.model().leases().window_of(LeaseId(lease)),
        Some(under)
    );
    assert_eq!(
        fixture.end_lease(&callers::shell(), lease),
        Err(ConnectRefusal::RoleNotAllowed),
        "the shell may not end the agent's lease"
    );
}

/// A run about to send keys has its leased window keyed: it comes to the front and takes the
/// keyboard though nothing else moved focus. The lease must be open (an ended or unknown one is
/// refused) and only the agent tier may ask.
#[test]
fn a_lease_keys_its_window_while_it_is_open() {
    let (mut fixture, [(_, under), (_, over)], _kept) = two_windows();
    let agent = callers::agent_tier();
    let (lease, _socket) = fixture
        .lease_window(&agent, &identifier_of(under), 7)
        .expect("granted");
    assert_eq!(
        fixture.focus_lease(&callers::shell(), lease),
        Err(ConnectRefusal::RoleNotAllowed)
    );
    assert_eq!(
        fixture.focus_lease(&agent, lease + 100),
        Err(ConnectRefusal::NoSuchLease)
    );
    assert_ne!(fixture.model().focus().key, Some(under));
    assert_eq!(fixture.focus_lease(&agent, lease), Ok(()));
    assert_eq!(fixture.model().focus().key, Some(under));
    assert_ne!(fixture.model().focus().key, Some(over));
    assert_eq!(fixture.end_lease(&agent, lease), Ok(()));
    assert_eq!(
        fixture.focus_lease(&agent, lease),
        Err(ConnectRefusal::NoSuchLease),
        "an ended lease keys nothing"
    );
}

/// The shell's own windows are never leased.
#[test]
fn a_shell_window_is_not_leased() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let id = fixture.connect(ConnectionRole::Shell);
    fixture.pump();
    let base = WmBase::bind(fixture.client(id), 7);
    let _top = map_toplevel(&mut fixture, id, &base, (300, 200));
    let window = windows(&fixture)[0];
    assert_eq!(
        fixture
            .lease_window(&callers::agent_tier(), &identifier_of(window), 7)
            .err(),
        Some(ConnectRefusal::NotLeasable)
    );
}

/// A press the lease made is let go even after the lease ended, so the button is never left down
/// in the client.
#[test]
fn a_release_still_reaches_the_client_after_its_lease_ended() {
    let (mut fixture, [(under_id, under), (_, over)], _kept) = two_windows();
    let agent = callers::agent_tier();
    let (lease, _socket) = fixture
        .lease_window(&agent, &identifier_of(under), 7)
        .expect("granted");
    let at = uncovered(rect_of(&fixture, under), rect_of(&fixture, over));
    // The first click only brings the window forward (it is not the key window yet).
    click(&mut fixture, lease, at);
    let before = count(fixture.client(under_id), "wl_pointer#", "button");
    send(
        &mut fixture,
        lease,
        InputKind::Button {
            button: BTN_LEFT,
            edge: Edge::Press,
        },
    );
    assert_eq!(fixture.end_lease(&agent, lease), Ok(()));
    send(
        &mut fixture,
        lease,
        InputKind::Button {
            button: BTN_LEFT,
            edge: Edge::Release,
        },
    );
    assert_eq!(
        count(fixture.client(under_id), "wl_pointer#", "button"),
        before + 2,
        "the press and its release"
    );
}

/// A press on the leased window's own title bar is the compositor's, not the window's: the lease
/// cannot drag the window about the desktop.
#[test]
fn a_lease_cannot_drag_its_window_by_the_title_bar() {
    let (mut fixture, [(_, under), (_, over)], _kept) = two_windows();
    let (lease, _socket) = fixture
        .lease_window(&callers::agent_tier(), &identifier_of(under), 7)
        .expect("granted");
    // Bring it to the front first, so its title bar shows whole.
    let first = uncovered(rect_of(&fixture, under), rect_of(&fixture, over));
    click(&mut fixture, lease, first);
    settle(&mut fixture);
    let rect = rect_of(&fixture, under);
    let bar = LogicalPoint {
        x: rect.origin.x + rect.size.w as i32 / 2,
        y: rect.origin.y + 8,
    };
    send(&mut fixture, lease, InputKind::MotionTo { at: bar });
    send(
        &mut fixture,
        lease,
        InputKind::Button {
            button: BTN_LEFT,
            edge: Edge::Press,
        },
    );
    send(
        &mut fixture,
        lease,
        InputKind::MotionTo {
            at: LogicalPoint {
                x: bar.x + 200,
                y: bar.y + 150,
            },
        },
    );
    send(
        &mut fixture,
        lease,
        InputKind::Button {
            button: BTN_LEFT,
            edge: Edge::Release,
        },
    );
    settle(&mut fixture);
    assert_eq!(
        rect_of(&fixture, under),
        rect,
        "the window stays where it was"
    );
}

/// The lease clicks its own window where it shows, and a click over the other window, which
/// covers part of it, never reaches either; after `EndLease` its clicks reach nothing.
#[test]
fn a_lease_reaches_its_window_and_no_other() {
    let (mut fixture, [(under_id, under), (over_id, over)], _kept) = two_windows();
    let agent = callers::agent_tier();
    let (lease, _socket) = fixture
        .lease_window(&agent, &identifier_of(under), 7)
        .expect("granted");
    let (under_rect, over_rect) = (rect_of(&fixture, under), rect_of(&fixture, over));

    let over_buttons = count(fixture.client(over_id), "wl_pointer#", "button");
    click(&mut fixture, lease, over_rect.centre());
    assert_eq!(
        count(fixture.client(over_id), "wl_pointer#", "button"),
        over_buttons,
        "another window never hears the lease"
    );

    // The first click only brings the window forward (it is not the key window yet).
    click(&mut fixture, lease, uncovered(under_rect, over_rect));
    let under_buttons = count(fixture.client(under_id), "wl_pointer#", "button");
    click(&mut fixture, lease, uncovered(under_rect, over_rect));
    assert_eq!(
        count(fixture.client(under_id), "wl_pointer#", "button"),
        under_buttons + 2,
        "the leased window hears the press and the release"
    );

    assert_eq!(fixture.end_lease(&agent, lease), Ok(()));
    assert!(fixture.model().leases().window_of(LeaseId(lease)).is_none());
    let under_buttons = count(fixture.client(under_id), "wl_pointer#", "button");
    click(&mut fixture, lease, uncovered(under_rect, over_rect));
    assert_eq!(
        count(fixture.client(under_id), "wl_pointer#", "button"),
        under_buttons,
        "an ended lease reaches nothing"
    );
    assert_eq!(fixture.end_lease(&agent, lease), Ok(()), "ending twice");
}

/// A press of the lease that the desktop held back (here, on the window's title bar) never
/// takes the person's own release of the same button: their click after it is whole.
#[test]
fn a_held_back_lease_press_leaves_the_persons_click_whole() {
    let (mut fixture, [(under_id, under), (_, over)], _kept) = two_windows();
    let (lease, _socket) = fixture
        .lease_window(&callers::agent_tier(), &identifier_of(under), 7)
        .expect("granted");
    let first = uncovered(rect_of(&fixture, under), rect_of(&fixture, over));
    click(&mut fixture, lease, first);
    settle(&mut fixture);
    let rect = rect_of(&fixture, under);
    let bar = LogicalPoint {
        x: rect.origin.x + rect.size.w as i32 / 2,
        y: rect.origin.y + 8,
    };
    send(&mut fixture, lease, InputKind::MotionTo { at: bar });
    send(
        &mut fixture,
        lease,
        InputKind::Button {
            button: BTN_LEFT,
            edge: Edge::Press,
        },
    );
    let mouse = fixture.input().plug(
        DeviceClass::Pointer,
        TestOrigin::Physical(DeviceClass::Pointer),
    );
    fixture.input().pointer_to(mouse, first);
    let before = count(fixture.client(under_id), "wl_pointer#", "button");
    fixture.input().button(mouse, BTN_LEFT, Edge::Press);
    fixture.input().button(mouse, BTN_LEFT, Edge::Release);
    fixture.pump();
    assert_eq!(
        count(fixture.client(under_id), "wl_pointer#", "button"),
        before + 2,
        "the window hears the person's press and release"
    );
}

const KEY_A: KeyCode = KeyCode(30);

/// How many keys the client's keyboards were sent.
fn keys_heard(fixture: &mut Fixture, id: ClientId) -> usize {
    count(fixture.client(id), "wl_keyboard#", "key")
}

/// A lease on the key window with a keyboard bound for it, while the shell's exclusive layer
/// holds the keyboard (as the confirm sheet does while it takes the person's answer). The lease
/// sends A, pressed and released. Returns the window's client, the shell's, the layer, and what
/// keeps the windows mapped.
fn key_while_a_layer_holds_the_keyboard() -> (Fixture, ClientId, ClientId, LayerSurface, Kept) {
    let (mut fixture, [_, (over_id, over)], kept) = two_windows();
    let (lease, _socket) = fixture
        .lease_window(&callers::agent_tier(), &identifier_of(over), 7)
        .expect("granted");
    let _keys = Keyboard::bind(fixture.client(over_id)).expect("a keyboard");
    // The person clicks the window: it has the keyboard before the sheet takes it.
    let rect = rect_of(&fixture, over);
    let mouse = fixture.input().plug(
        DeviceClass::Pointer,
        TestOrigin::Physical(DeviceClass::Pointer),
    );
    let middle = LogicalPoint {
        x: rect.origin.x + rect.size.w as i32 / 2,
        y: rect.origin.y + rect.size.h as i32 / 2,
    };
    fixture.input().pointer_to(mouse, middle);
    fixture.input().button(mouse, BTN_LEFT, Edge::Press);
    fixture.input().button(mouse, BTN_LEFT, Edge::Release);
    fixture.pump();
    let shell = fixture.connect(ConnectionRole::Shell);
    fixture.pump();
    let _shell_keys = Keyboard::bind(fixture.client(shell)).expect("a keyboard");
    let sheet = LayerSurface::create(
        fixture.client(shell),
        &LayerAsk {
            level: Layer::Overlay,
            namespace: "sheet",
            anchor: Anchor::empty(),
            exclusive_zone: 0,
            keyboard: KeyboardInteractivity::Exclusive,
            size: (400, 300),
        },
    )
    .expect("layer");
    fixture.pump();
    for edge in [Edge::Press, Edge::Release] {
        send(&mut fixture, lease, InputKind::Key { code: KEY_A, edge });
    }
    (fixture, over_id, shell, sheet, kept)
}

/// A key of the lease that finds the keyboard on a surface outside the lease waits, and goes to
/// the leased window, press then release, once the keyboard is back there.
#[test]
fn a_lease_key_waits_for_the_keyboard_and_goes_when_it_returns() {
    let (mut fixture, window, shell, sheet, _kept) = key_while_a_layer_holds_the_keyboard();
    assert_eq!(
        keys_heard(&mut fixture, window),
        0,
        "not while the sheet has the keyboard"
    );
    fixture.advance(Duration::from_millis(100));
    assert_eq!(
        keys_heard(&mut fixture, window),
        0,
        "still waiting inside the bound"
    );
    sheet.destroy(fixture.client(shell));
    fixture.advance(Duration::from_millis(100));
    assert_eq!(
        keys_heard(&mut fixture, window),
        2,
        "the press and the release reach the leased window"
    );
}

/// A key that waited the whole bound without the keyboard coming back is dropped, and its release
/// with it: nothing reaches the window later.
#[test]
fn a_lease_key_is_dropped_after_the_bound_when_the_keyboard_stays_elsewhere() {
    let (mut fixture, window, shell, sheet, _kept) = key_while_a_layer_holds_the_keyboard();
    fixture.advance(Duration::from_millis(600));
    sheet.destroy(fixture.client(shell));
    fixture.advance(Duration::from_millis(100));
    assert_eq!(
        keys_heard(&mut fixture, window),
        0,
        "dropped, not delivered late"
    );
}

/// How many buttons the client's pointers were sent.
fn buttons_heard(fixture: &mut Fixture, id: ClientId) -> usize {
    count(fixture.client(id), "wl_pointer#", "button")
}

/// A lease on the top window while the shell's sheet covers the output (as a confirm sheet does
/// until it is unmapped after the person's answer). The lease clicks the middle of the window.
/// Returns the window's client, the shell's, the sheet, and what keeps the windows mapped.
fn click_while_a_sheet_covers_the_window() -> (Fixture, ClientId, ClientId, LayerSurface, Kept) {
    let (mut fixture, [_, (over_id, over)], kept) = two_windows();
    let (lease, _socket) = fixture
        .lease_window(&callers::agent_tier(), &identifier_of(over), 7)
        .expect("granted");
    let shell = fixture.connect(ConnectionRole::Shell);
    fixture.pump();
    let sheet = LayerSurface::create(
        fixture.client(shell),
        &LayerAsk {
            level: Layer::Overlay,
            namespace: "sheet",
            anchor: Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right,
            exclusive_zone: 0,
            keyboard: KeyboardInteractivity::None,
            size: (0, 0),
        },
    )
    .expect("layer");
    fixture.pump();
    let rect = rect_of(&fixture, over);
    let middle = LogicalPoint {
        x: rect.origin.x + rect.size.w as i32 / 2,
        y: rect.origin.y + rect.size.h as i32 / 2,
    };
    click(&mut fixture, lease, middle);
    fixture.pump();
    (fixture, over_id, shell, sheet, kept)
}

/// A click of the lease that lands on the shell's sheet waits, and reaches the leased window,
/// press then release, once the sheet is gone.
#[test]
fn a_lease_click_on_the_shells_sheet_waits_and_goes_when_it_closes() {
    let (mut fixture, window, shell, sheet, _kept) = click_while_a_sheet_covers_the_window();
    assert_eq!(
        buttons_heard(&mut fixture, window),
        0,
        "not while the sheet covers the window"
    );
    fixture.advance(Duration::from_millis(100));
    assert_eq!(
        buttons_heard(&mut fixture, window),
        0,
        "still waiting inside the bound"
    );
    sheet.destroy(fixture.client(shell));
    fixture.advance(Duration::from_millis(100));
    assert_eq!(
        buttons_heard(&mut fixture, window),
        2,
        "the press and the release reach the leased window"
    );
}

/// A click that waited the whole bound with the sheet still up is dropped: nothing reaches the
/// window once it closes.
#[test]
fn a_lease_click_is_dropped_after_the_bound_when_the_sheet_stays() {
    let (mut fixture, window, shell, sheet, _kept) = click_while_a_sheet_covers_the_window();
    fixture.advance(Duration::from_millis(600));
    sheet.destroy(fixture.client(shell));
    fixture.advance(Duration::from_millis(100));
    assert_eq!(
        buttons_heard(&mut fixture, window),
        0,
        "dropped, not delivered late"
    );
}
