//! Input to a popup that holds the seat's grab: the pointer reaches it where it is drawn, and
//! when its grab ends the keyboard goes where the model says, not to the surface the popup hangs
//! from. The shell's bar is a layer surface that never takes the keyboard; its menu is an xdg
//! popup with an explicit grab, made after a click on the bar.

use super::xdg_shell::{click, map_toplevel, pointer, rect_of, settle, windows};
use casement_core::DeviceId;
use casement_core::geometry::{LogicalPoint, PhysicalSize};
use casement_core::input::{ButtonCode, DeviceClass, Edge, KeyCode};
use casement_core::policy::ConnectionRole;
use casement_testclient::seat::keyboard::Keyboard;
use casement_testclient::surface::layer::{
    Anchor, KeyboardInteractivity, Layer, LayerAsk, LayerSurface,
};
use casement_testclient::surface::popup::{Popup, PopupSpec};
use casement_testclient::surface::toplevel::{BoundPointer, WmBase, bind_pointer};
use casement_testclient::{Pattern, TestClient, shm_buffer};
use casement_testkit::{ClientId, Fixture, FixtureSpec, TestOrigin};
use wayland_client::Proxy;
use wayland_client::protocol::wl_surface::WlSurface;

const KEY_ESC: KeyCode = KeyCode(1);
const KEY_A: KeyCode = KeyCode(30);
const LEFT: ButtonCode = ButtonCode(0x110);

/// Where the bar's menu is: 100 x 50, its top-left 30 below the bar's top-left corner, 100
/// right of it, which is where the bar's click lands.
const MENU_BOX: (i32, i32, i32, i32) = (100, 30, 100, 50);

/// A bar along the top: 30 high, no keyboard.
fn bar_ask() -> LayerAsk {
    LayerAsk {
        level: Layer::Top,
        namespace: "bar",
        anchor: Anchor::Top | Anchor::Left | Anchor::Right,
        exclusive_zone: 30,
        keyboard: KeyboardInteractivity::None,
        size: (0, 30),
    }
}

/// Maps `ask` the way a shell does: create, wait for the configure, acknowledge it and attach
/// a buffer of the configured size.
fn map_layer(fixture: &mut Fixture, id: ClientId, ask: &LayerAsk) -> LayerSurface {
    let layer = LayerSurface::create(fixture.client(id), ask).expect("a layer surface");
    fixture.pump();
    let configure = layer.configure().expect("the initial configure");
    assert!(layer.ack(fixture.client(id)));
    let size = PhysicalSize {
        w: configure.width.max(ask.size.0),
        h: configure.height.max(ask.size.1),
    };
    let buffer = shm_buffer(fixture.client(id), size, Pattern::Solid(0xff20_2020));
    layer.show(fixture.client(id), &buffer);
    fixture.pump();
    layer
}

/// How a surface reads in a transcript line's arguments.
fn named(surface: &WlSurface) -> String {
    format!("wl_surface#{}", surface.id().protocol_id())
}

/// The events of `client`'s keyboard, as `(event, args)`.
fn keyboard_events(client: &TestClient) -> Vec<(String, Vec<String>)> {
    client
        .transcript()
        .0
        .iter()
        .filter(|line| line.object.starts_with("wl_keyboard#"))
        .map(|line| (line.event.clone(), line.args.clone()))
        .collect()
}

/// How many times `client`'s keyboard entered `surface`.
fn enters_of(client: &TestClient, surface: &WlSurface) -> usize {
    let name = named(surface);
    keyboard_events(client)
        .iter()
        .filter(|(event, args)| event == "enter" && args.iter().any(|arg| arg.contains(&name)))
        .count()
}

/// How many key events `client`'s keyboard has received.
fn keys_of(client: &TestClient) -> usize {
    keyboard_events(client)
        .iter()
        .filter(|(event, _)| event == "key")
        .count()
}

/// The desk of the shell-host script: an app window with the keyboard, the shell's bar along the
/// top, and a menu of the bar that holds the grab.
struct Desk {
    fixture: Fixture,
    app: ClientId,
    shell: ClientId,
    bar: LayerSurface,
    popup: Popup,
    presses: BoundPointer,
    base: WmBase,
    mouse: DeviceId,
    keys: DeviceId,
}

impl Desk {
    /// Opens the menu: a click on the bar, then the popup grab answering its press.
    fn open() -> Desk {
        let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
        let keys = fixture.input().plug(
            DeviceClass::Keyboard,
            TestOrigin::Physical(DeviceClass::Keyboard),
        );
        let mouse = pointer(&mut fixture);
        let app = fixture.connect(ConnectionRole::Public);
        let shell = fixture.connect(ConnectionRole::Public);
        fixture.pump();
        let _app_keyboard = Keyboard::bind(fixture.client(app)).expect("the app's keyboard");
        let _shell_keyboard = Keyboard::bind(fixture.client(shell)).expect("the shell's keyboard");
        let presses = bind_pointer(fixture.client(shell));
        fixture.pump();
        // The app maps first and takes the keyboard; the bar never asks for it.
        let app_base = WmBase::bind(fixture.client(app), 7);
        let _window = map_toplevel(&mut fixture, app, &app_base, (400, 300));
        settle(&mut fixture);
        // A window that maps takes no focus (A7): the first click activates it, and is swallowed.
        let window = windows(&fixture)[0];
        click(&mut fixture, mouse, window);
        fixture.pump();
        let bar = map_layer(&mut fixture, shell, &bar_ask());
        // The app holds the keyboard before the menu opens.
        let typed = keys_of(fixture.client(app));
        fixture.input().key(keys, KEY_A, Edge::Press);
        fixture.input().key(keys, KEY_A, Edge::Release);
        fixture.pump();
        assert!(
            keys_of(fixture.client(app)) > typed,
            "the app holds the keyboard"
        );
        // A person clicks the bar's menu button.
        fixture
            .input()
            .pointer_to(mouse, LogicalPoint { x: 110, y: 10 });
        fixture.input().button(mouse, LEFT, Edge::Press);
        fixture.input().button(mouse, LEFT, Edge::Release);
        fixture.pump();
        let serial = presses.last_press().expect("the bar's press reached it");
        let base = WmBase::bind(fixture.client(shell), 7);
        let surface = fixture.client(shell).create_surface().expect("a surface");
        let spec = PopupSpec {
            size: (MENU_BOX.2, MENU_BOX.3),
            anchor_rect: (MENU_BOX.0, MENU_BOX.1, 1, 1),
            ..PopupSpec::menu_at((0, 0))
        };
        let popup = Popup::of_layer(fixture.client(shell), &base, surface, spec);
        bar.adopt(&popup.popup);
        popup.grab(&presses.seat, serial);
        popup.surface.commit();
        fixture.pump();
        assert!(popup.ack_latest().is_some(), "the popup was configured");
        let buffer = shm_buffer(
            fixture.client(shell),
            PhysicalSize {
                w: MENU_BOX.2 as u32,
                h: MENU_BOX.3 as u32,
            },
            Pattern::Solid(0xff40_4040),
        );
        popup.show(&buffer);
        fixture.pump();
        assert_eq!(popup.log.with(|seen| seen.done), 0, "the popup is open");
        Desk {
            fixture,
            app,
            shell,
            bar,
            popup,
            presses,
            base,
            mouse,
            keys,
        }
    }

    /// A key, pressed and released.
    fn tap(&mut self, code: KeyCode) {
        self.fixture.input().key(self.keys, code, Edge::Press);
        self.fixture.input().key(self.keys, code, Edge::Release);
        self.fixture.pump();
    }

    fn app_keys(&mut self) -> usize {
        keys_of(self.fixture.client(self.app))
    }

    fn shell_keys(&mut self) -> usize {
        keys_of(self.fixture.client(self.shell))
    }

    /// A person types `a`; whether the app got it (and the shell did not).
    fn app_gets_a_typed_key(&mut self) -> bool {
        let (app, shell) = (self.app_keys(), self.shell_keys());
        self.tap(KEY_A);
        self.app_keys() > app && self.shell_keys() == shell
    }
}

/// A click inside a popup of a layer surface reaches the popup: the popup is in what the pointer
/// hits, where it is drawn, and its grab is not dismissed by a press on itself.
#[test]
fn a_click_inside_a_bar_popup_reaches_the_popup() {
    let mut desk = Desk::open();
    let before = desk.presses.log.with(|seen| seen.buttons.len());
    let inside = LogicalPoint {
        x: MENU_BOX.0 + MENU_BOX.2 / 2,
        y: MENU_BOX.1 + MENU_BOX.3 / 2,
    };
    desk.fixture.input().pointer_to(desk.mouse, inside);
    desk.fixture.input().button(desk.mouse, LEFT, Edge::Press);
    desk.fixture.input().button(desk.mouse, LEFT, Edge::Release);
    desk.fixture.pump();
    assert_eq!(
        desk.popup.log.with(|seen| seen.done),
        0,
        "a press inside the menu does not dismiss it"
    );
    assert!(
        desk.presses.log.with(|seen| seen.buttons.len()) > before,
        "the menu's client was given the press inside the menu"
    );
}

/// The same press a little left of the menu's box is on the bar, not the menu: still the
/// shell's, so the menu stays (the grab's client is the shell).
#[test]
fn a_press_on_the_bar_beside_the_popup_does_not_dismiss_it() {
    let mut desk = Desk::open();
    desk.fixture
        .input()
        .pointer_to(desk.mouse, LogicalPoint { x: 40, y: 10 });
    desk.fixture.input().button(desk.mouse, LEFT, Edge::Press);
    desk.fixture.input().button(desk.mouse, LEFT, Edge::Release);
    desk.fixture.pump();
    assert_eq!(desk.popup.log.with(|seen| seen.done), 0);
}

/// A popup of a toplevel is hit where it hangs outside its parent's box too.
#[test]
fn a_click_inside_a_toplevel_popup_reaches_the_popup() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let id = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let base = WmBase::bind(fixture.client(id), 7);
    let parent = map_toplevel(&mut fixture, id, &base, (400, 300));
    let mouse = pointer(&mut fixture);
    let presses: BoundPointer = bind_pointer(fixture.client(id));
    fixture.pump();
    settle(&mut fixture);
    let window = windows(&fixture)[0];
    let origin = rect_of(&fixture, window).origin;
    // A press on the window, to answer with the popup's grab.
    // The first click activates the window and is swallowed; the second is delivered.
    click(&mut fixture, mouse, window);
    click(&mut fixture, mouse, window);
    fixture.pump();
    let serial = presses.last_press().expect("the window's press");
    let surface = fixture.client(id).create_surface().expect("a surface");
    let popup = Popup::of(
        fixture.client(id),
        &base,
        &parent,
        surface,
        PopupSpec::menu_at((380, 10)),
    );
    popup.grab(&presses.seat, serial);
    popup.surface.commit();
    fixture.pump();
    popup.ack_latest().expect("the popup was configured");
    let buffer = shm_buffer(
        fixture.client(id),
        PhysicalSize { w: 100, h: 50 },
        Pattern::Solid(0xff40_4040),
    );
    popup.show(&buffer);
    fixture.pump();
    let before = presses.log.with(|seen| seen.buttons.len());
    // The popup hangs out to the right of the 400 wide parent: this point is only the popup's.
    let inside = LogicalPoint {
        x: origin.x + 430,
        y: origin.y + 30,
    };
    fixture.input().pointer_to(mouse, inside);
    fixture.input().button(mouse, LEFT, Edge::Press);
    fixture.input().button(mouse, LEFT, Edge::Release);
    fixture.pump();
    assert_eq!(popup.log.with(|seen| seen.done), 0, "the popup stays open");
    assert!(
        presses.log.with(|seen| seen.buttons.len()) > before,
        "the popup was given the press"
    );
}

/// Escape closes the menu (the client destroys it); the next key goes to the app, which the
/// model says holds the keyboard, and not to the bar the menu hung from.
#[test]
fn closing_a_bar_popup_returns_the_keyboard_to_the_app() {
    let mut desk = Desk::open();
    let (app, shell) = (desk.app_keys(), desk.shell_keys());
    desk.tap(KEY_ESC);
    assert!(
        desk.shell_keys() > shell && desk.app_keys() == app,
        "keys reach the open menu, not the app"
    );
    desk.popup.destroy();
    desk.fixture.pump();
    assert!(
        desk.app_gets_a_typed_key(),
        "after the menu closed the app gets the keys"
    );
}

/// The shell dies with its menu open: smithay drops the grab before the popup's destruction
/// reaches casement, and the keyboard must still go back to the app the model says holds it.
#[test]
fn a_client_that_disconnects_with_its_popup_open_returns_the_keyboard_to_the_app() {
    let mut desk = Desk::open();
    desk.fixture.disconnect(desk.shell);
    desk.fixture.pump();
    let typed = desk.app_keys();
    desk.tap(KEY_A);
    assert!(
        desk.app_keys() > typed,
        "after the menu's client left the app gets the keys"
    );
}

/// A press outside the menu dismisses it and leaves the keyboard with the app.
#[test]
fn clicking_outside_a_bar_popup_returns_the_keyboard_to_the_app() {
    let mut desk = Desk::open();
    let window = windows(&desk.fixture)[0];
    click(&mut desk.fixture, desk.mouse, window);
    desk.fixture.pump();
    assert_eq!(
        desk.popup.log.with(|seen| seen.done),
        1,
        "a press outside the menu dismisses it"
    );
    assert!(
        desk.app_gets_a_typed_key(),
        "after the menu was dismissed the app gets the keys"
    );
}

/// A layer surface that asked for no keyboard is never given wl_keyboard.enter, not while a
/// menu hangs from it and not when the menu's grab ends, by a press outside or by the client.
#[test]
fn a_layer_without_keyboard_interactivity_is_never_keyed() {
    let mut desk = Desk::open();
    // The menu is dismissed by a press outside it.
    let window = windows(&desk.fixture)[0];
    click(&mut desk.fixture, desk.mouse, window);
    desk.fixture.pump();
    assert_eq!(
        enters_of(desk.fixture.client(desk.shell), desk.bar.surface()),
        0
    );
    // A second menu, closed by the client.
    let mut desk2 = Desk::open();
    desk2.tap(KEY_ESC);
    desk2.popup.destroy();
    desk2.fixture.pump();
    desk2.tap(KEY_A);
    assert_eq!(
        enters_of(desk2.fixture.client(desk2.shell), desk2.bar.surface()),
        0
    );
}

/// A submenu that closes hands the grab back to its menu, not to the app: the keys still reach
/// the menu until it closes too.
#[test]
fn closing_a_submenu_leaves_the_keyboard_with_its_menu() {
    let mut desk = Desk::open();
    // A press inside the menu answers the submenu's grab.
    let inside = LogicalPoint {
        x: MENU_BOX.0 + 10,
        y: MENU_BOX.1 + 10,
    };
    desk.fixture.input().pointer_to(desk.mouse, inside);
    desk.fixture.input().button(desk.mouse, LEFT, Edge::Press);
    desk.fixture.input().button(desk.mouse, LEFT, Edge::Release);
    desk.fixture.pump();
    let serial = desk.presses.last_press().expect("the menu's press");
    let surface = desk
        .fixture
        .client(desk.shell)
        .create_surface()
        .expect("a surface");
    let submenu = Popup::of_popup(
        desk.fixture.client(desk.shell),
        &desk.base,
        &desk.popup,
        surface,
        PopupSpec::menu_at((20, 20)),
    );
    submenu.grab(&desk.presses.seat, serial);
    submenu.surface.commit();
    desk.fixture.pump();
    submenu.ack_latest().expect("the submenu was configured");
    let buffer = shm_buffer(
        desk.fixture.client(desk.shell),
        PhysicalSize { w: 100, h: 50 },
        Pattern::Solid(0xff60_6060),
    );
    submenu.show(&buffer);
    desk.fixture.pump();
    assert_eq!(submenu.log.with(|seen| seen.done), 0, "the submenu is open");
    submenu.destroy();
    desk.fixture.pump();
    assert!(
        !desk.app_gets_a_typed_key(),
        "the menu still holds the keyboard"
    );
    desk.popup.destroy();
    desk.fixture.pump();
    assert!(
        desk.app_gets_a_typed_key(),
        "the app has it once the menu is gone"
    );
}
