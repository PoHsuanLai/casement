//! `layer_shell` (P0-H3).

use casement_core::OutputName;
use casement_core::geometry::{LogicalPoint, PhysicalSize};
use casement_core::input::{DeviceClass, Edge, KeyCode};
use casement_core::policy::ConnectionRole;
use casement_testclient::seat::keyboard::Keyboard;
use casement_testclient::seat::pointer::Pointer;
use casement_testclient::surface::layer::{
    Anchor, KeyboardInteractivity, Layer, LayerAsk, LayerSurface,
};
use casement_testclient::surface::subsurface::{Compositor, H1Data};
use casement_testclient::surface::toplevel::{State, WmBase};
use casement_testclient::{Pattern, TestClient, shm_buffer};
use casement_testkit::{ClientId, Fixture, FixtureSpec, FrameCount, ShotTarget, TestOrigin};

use crate::protocol::xdg_shell::{make_key, map_toplevel, settle, windows};

/// The evdev code of the left shift key.
const KEY_LEFTSHIFT: KeyCode = KeyCode(42);

/// A layer that spans the whole output, takes no keyboard and sits below everything.
fn wallpaper() -> LayerAsk {
    LayerAsk {
        level: Layer::Background,
        namespace: "wallpaper",
        anchor: Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right,
        exclusive_zone: -1,
        keyboard: KeyboardInteractivity::None,
        size: (0, 0),
    }
}

/// The events of `object_prefix` objects in `client`'s transcript, as `(event, args)`.
fn events_of(client: &TestClient, object_prefix: &str) -> Vec<(String, Vec<String>)> {
    client
        .transcript()
        .0
        .iter()
        .filter(|line| line.object.starts_with(object_prefix))
        .map(|line| (line.event.clone(), line.args.clone()))
        .collect()
}

/// Maps `ask` the way a shell does: create, wait for the configure, acknowledge it and attach
/// a buffer of the configured size.
pub(crate) fn map(fixture: &mut Fixture, id: ClientId, ask: &LayerAsk) -> LayerSurface {
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

/// An Exclusive layer gets wl_keyboard.enter with the current modifiers on the commit that maps it (A16).
#[test]
fn an_exclusive_layer_takes_the_keyboard_on_its_mapping_commit() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let device = fixture.input().plug(
        DeviceClass::Keyboard,
        TestOrigin::Physical(DeviceClass::Keyboard),
    );
    // A person is holding shift when the layer maps.
    fixture.input().key(device, KEY_LEFTSHIFT, Edge::Press);
    // Exclusive keyboard is the shell's (CASC-02); another client's waits for a press.
    let id = fixture.connect(ConnectionRole::Shell);
    fixture.pump();
    let _keyboard = Keyboard::bind(fixture.client(id)).expect("a keyboard");
    let launcher = LayerAsk {
        level: Layer::Overlay,
        namespace: "launcher",
        anchor: Anchor::empty(),
        exclusive_zone: 0,
        keyboard: KeyboardInteractivity::Exclusive,
        size: (400, 300),
    };
    // Only the first commit, the one that asks for the initial configure, has been made.
    let layer = LayerSurface::create(fixture.client(id), &launcher).expect("a layer surface");
    fixture.pump();
    assert!(layer.configure().is_some(), "the initial configure arrived");
    let events = events_of(fixture.client(id), "wl_keyboard#");
    let enter = events
        .iter()
        .position(|(event, _)| event == "enter")
        .expect("the keyboard entered the layer on the mapping commit");
    let (event, args) = &events[enter + 1];
    assert_eq!(event, "modifiers", "modifiers follow the enter at once");
    assert_ne!(
        args[1], "0",
        "shift is depressed in the modifiers sent with the enter"
    );
}

/// A layer mapping under a pointer that does not move still receives enter (SP1 A1).
#[test]
fn a_layer_under_a_resting_pointer_gets_enter() {
    use casement_core::geometry::LogicalPoint;
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let mouse = fixture.input().plug(
        DeviceClass::Pointer,
        TestOrigin::Physical(DeviceClass::Pointer),
    );
    fixture
        .input()
        .pointer_to(mouse, LogicalPoint { x: 400, y: 300 });
    let id = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let _pointer = Pointer::bind(fixture.client(id)).expect("a pointer");
    let _layer = map(&mut fixture, id, &wallpaper());
    // The pointer never moved after the layer mapped, and yet it entered.
    let events = events_of(fixture.client(id), "wl_pointer#");
    assert!(
        events.iter().any(|(event, _)| event == "enter"),
        "the surface now under the resting pointer received enter: {events:?}"
    );
}

/// The pointer starts at the centre of the primary output, not at its top-left corner, where it
/// would dwell in a hot corner at start-up.
#[test]
fn the_pointer_starts_at_the_centre_of_the_primary_output() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let _mouse = fixture.input().plug(
        DeviceClass::Pointer,
        TestOrigin::Physical(DeviceClass::Pointer),
    );
    let id = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let _pointer = Pointer::bind(fixture.client(id)).expect("a pointer");
    let _layer = map(&mut fixture, id, &wallpaper());
    let enters: Vec<_> = events_of(fixture.client(id), "wl_pointer#")
        .into_iter()
        .filter(|(event, _)| event == "enter")
        .collect();
    assert_eq!(
        enters.last().map(|(_, args)| args[2..].to_vec()),
        Some(vec!["960.000".to_owned(), "540.000".to_owned()]),
        "the surface under the resting pointer is entered at the output's centre: {enters:?}"
    );
}

/// A bar's exclusive zone is subtracted from a maximised window's configure (A16).
#[test]
fn exclusive_zones_shrink_maximised_windows() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let id = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let base = WmBase::bind(fixture.client(id), 6);
    let window = map_toplevel(&mut fixture, id, &base, (400, 300));
    // With no bar the output's whole 1920x1080 is usable.
    window.set_maximized();
    fixture.pump();
    assert!(window.ack_latest().is_some());
    let whole = window.configures().last().expect("a configure").clone();
    assert_eq!(whole.size, (1920, 1080));
    assert!(whole.states.contains(&State::Maximized));
    // A 28 px bar on the top edge reserves its zone: the window shrinks to what is left.
    let bar = LayerAsk {
        level: Layer::Top,
        namespace: "bar",
        anchor: Anchor::Top | Anchor::Left | Anchor::Right,
        exclusive_zone: 28,
        keyboard: KeyboardInteractivity::None,
        size: (0, 28),
    };
    let _bar = map(&mut fixture, id, &bar);
    assert!(window.ack_latest().is_some());
    let shrunk = window.configures().last().expect("a configure").clone();
    assert_eq!(
        shrunk.size,
        (1920, 1080 - 28),
        "the bar's zone is subtracted: {shrunk:?}"
    );
    assert!(shrunk.states.contains(&State::Maximized));
    // The window now fills the area below the bar.
    let model_window = *fixture.model().windows().keys().next().expect("a window");
    let rect = crate::protocol::xdg_shell::rect_of(&fixture, model_window);
    assert_eq!(
        (rect.origin.y, rect.size.h),
        (28, 1080 - 28),
        "it is drawn below the bar"
    );
}

/// wlr-layer-shell: zone 0 moves a surface out of other surfaces' exclusive zones. A surface held
/// to all four edges is sized to what the bar leaves, whichever maps first, and the bar's strip
/// is not under it.
#[test]
fn a_zone_zero_layer_keeps_clear_of_the_bar() {
    use casement_core::geometry::LogicalPoint;
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let mouse = fixture.input().plug(
        DeviceClass::Pointer,
        TestOrigin::Physical(DeviceClass::Pointer),
    );
    // The pointer rests inside the bar's strip.
    fixture
        .input()
        .pointer_to(mouse, LogicalPoint { x: 400, y: 20 });
    let shell = fixture.connect(ConnectionRole::Public);
    let centre = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let _shell_pointer = Pointer::bind(fixture.client(shell)).expect("a pointer");
    let _centre_pointer = Pointer::bind(fixture.client(centre)).expect("a pointer");
    let early = LayerAsk {
        level: Layer::Overlay,
        namespace: "notification-center",
        anchor: Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right,
        exclusive_zone: 0,
        keyboard: KeyboardInteractivity::None,
        size: (0, 0),
    };
    let first = LayerSurface::create(fixture.client(centre), &early).expect("a layer surface");
    fixture.pump();
    let before = first.configure().expect("the initial configure");
    assert_eq!((before.width, before.height), (1920, 1080), "no bar yet");
    let bar = LayerAsk {
        level: Layer::Top,
        namespace: "bar",
        anchor: Anchor::Top | Anchor::Left | Anchor::Right,
        exclusive_zone: 28,
        keyboard: KeyboardInteractivity::None,
        size: (0, 28),
    };
    let _bar = map(&mut fixture, shell, &bar);
    let after = first.configure().expect("a configure for the new zone");
    assert_eq!(
        (after.width, after.height),
        (1920, 1080 - 28),
        "the bar's zone arrived: the surface is configured again"
    );
    assert!(first.ack(fixture.client(centre)));
    let buffer = shm_buffer(
        fixture.client(centre),
        PhysicalSize { w: 1920, h: 1052 },
        Pattern::Solid(0xff20_2020),
    );
    first.show(fixture.client(centre), &buffer);
    fixture.pump();
    // A frame later (the surfaces' sizes are read per frame), the pointer moves inside the bar's strip.
    fixture.advance(std::time::Duration::from_millis(100));
    fixture
        .input()
        .pointer_to(mouse, LogicalPoint { x: 420, y: 22 });
    fixture.pump();
    let entered = |fixture: &mut Fixture, id| {
        events_of(fixture.client(id), "wl_pointer#")
            .iter()
            .any(|(event, _)| event == "enter")
    };
    assert!(entered(&mut fixture, shell), "the bar is under the pointer");
    assert!(
        !entered(&mut fixture, centre),
        "the zone-0 surface is below the bar, not over it"
    );
}

/// What the pointer entered, by client, after it moved to `at`.
fn pointer_enters_at(
    fixture: &mut Fixture,
    mouse: casement_core::DeviceId,
    at: (i32, i32),
    clients: &[ClientId],
) -> Vec<usize> {
    use casement_core::geometry::LogicalPoint;
    let count = |fixture: &mut Fixture, id: ClientId| {
        events_of(fixture.client(id), "wl_pointer#")
            .iter()
            .filter(|(event, _)| event == "enter")
            .count()
    };
    let before: Vec<usize> = clients.iter().map(|id| count(fixture, *id)).collect();
    fixture.advance(std::time::Duration::from_millis(100));
    fixture
        .input()
        .pointer_to(mouse, LogicalPoint { x: at.0, y: at.1 });
    fixture.pump();
    clients
        .iter()
        .zip(before)
        .map(|(id, before)| count(fixture, *id) - before)
        .collect()
}

/// A bar on the top edge of `zone` pixels, drawn `height` tall.
fn top_bar(zone: i32, height: u32) -> LayerAsk {
    LayerAsk {
        level: Layer::Top,
        namespace: "bar",
        anchor: Anchor::Top | Anchor::Left | Anchor::Right,
        exclusive_zone: zone,
        keyboard: KeyboardInteractivity::None,
        size: (0, height),
    }
}

/// wlr-layer-shell: each exclusive layer is arranged in what the earlier ones left. Two bars on
/// the top edge stack, and the pointer enters each where it is drawn: the second bar's strip
/// starts below the first, and the first bar's strip is not under the second.
#[test]
fn the_pointer_enters_the_second_bar_where_it_is_drawn() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let mouse = fixture.input().plug(
        DeviceClass::Pointer,
        TestOrigin::Physical(DeviceClass::Pointer),
    );
    let (first, second) = (
        fixture.connect(ConnectionRole::Public),
        fixture.connect(ConnectionRole::Public),
    );
    fixture.pump();
    let _pointers = [first, second].map(|id| Pointer::bind(fixture.client(id)).expect("a pointer"));
    let _first = map(&mut fixture, first, &top_bar(24, 24));
    let _second = map(&mut fixture, second, &top_bar(30, 30));
    let clients = [first, second];
    assert_eq!(
        pointer_enters_at(&mut fixture, mouse, (400, 10), &clients),
        [1, 0],
        "the first bar is at the top of the output"
    );
    assert_eq!(
        pointer_enters_at(&mut fixture, mouse, (420, 40), &clients),
        [0, 1],
        "the second bar is below the first"
    );
}

/// A bar on the top edge and then a dock on the left: the dock is arranged in what the bar left,
/// so it starts below the bar, in the hit test as it does on screen.
#[test]
fn a_dock_mapped_after_a_bar_is_hit_below_it() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let mouse = fixture.input().plug(
        DeviceClass::Pointer,
        TestOrigin::Physical(DeviceClass::Pointer),
    );
    let (bar, dock) = (
        fixture.connect(ConnectionRole::Public),
        fixture.connect(ConnectionRole::Public),
    );
    fixture.pump();
    let _pointers = [bar, dock].map(|id| Pointer::bind(fixture.client(id)).expect("a pointer"));
    let _bar = map(&mut fixture, bar, &top_bar(24, 24));
    let side = LayerAsk {
        level: Layer::Top,
        namespace: "dock",
        anchor: Anchor::Top | Anchor::Bottom | Anchor::Left,
        exclusive_zone: 64,
        keyboard: KeyboardInteractivity::None,
        size: (64, 0),
    };
    let dock_layer = LayerSurface::create(fixture.client(dock), &side).expect("a layer surface");
    fixture.pump();
    let configure = dock_layer.configure().expect("the initial configure");
    assert_eq!(
        (configure.width, configure.height),
        (64, 1080 - 24),
        "the dock keeps the width it asked for, its height is what the bar left"
    );
    assert!(dock_layer.ack(fixture.client(dock)));
    let buffer = shm_buffer(
        fixture.client(dock),
        PhysicalSize { w: 64, h: 1056 },
        Pattern::Solid(0xff20_2020),
    );
    dock_layer.show(fixture.client(dock), &buffer);
    fixture.pump();
    let clients = [bar, dock];
    assert_eq!(
        pointer_enters_at(&mut fixture, mouse, (10, 10), &clients),
        [1, 0],
        "the bar's strip is the bar's, the dock is not over it"
    );
    assert_eq!(
        pointer_enters_at(&mut fixture, mouse, (10, 60), &clients),
        [0, 1],
        "below the bar is the dock"
    );
}

/// A full-screen window covers the whole output, the bar's strip included: it is configured to
/// the output's size and drawn there, and the bar is not above it (a full-screen window is above
/// the top layer and below the overlay).
#[test]
fn a_full_screen_window_covers_the_whole_output_with_a_bar_mapped() {
    use casement_core::geometry::LogicalPoint;
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let mouse = fixture.input().plug(
        DeviceClass::Pointer,
        TestOrigin::Physical(DeviceClass::Pointer),
    );
    let app = fixture.connect(ConnectionRole::Public);
    let shell = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let _pointer = Pointer::bind(fixture.client(app)).expect("a pointer");
    let base = WmBase::bind(fixture.client(app), 6);
    let window = map_toplevel(&mut fixture, app, &base, (400, 300));
    let bar = LayerAsk {
        level: Layer::Top,
        namespace: "bar",
        anchor: Anchor::Top | Anchor::Left | Anchor::Right,
        exclusive_zone: 28,
        keyboard: KeyboardInteractivity::None,
        size: (0, 28),
    };
    let _bar = map(&mut fixture, shell, &bar);
    let window_id = windows(&fixture)[0];
    make_key(&mut fixture, window_id);
    window.set_fullscreen();
    fixture.pump();
    assert!(window.ack_latest().is_some());
    let told = window.configures().last().expect("a configure").clone();
    assert!(told.states.contains(&State::Fullscreen), "{told:?}");
    assert_eq!(
        told.size,
        (1920, 1080),
        "the whole output, not what the bar leaves: {told:?}"
    );
    let buffer = shm_buffer(
        fixture.client(app),
        PhysicalSize { w: 1920, h: 1080 },
        Pattern::Gradient,
    );
    window.show(&buffer);
    fixture.pump();
    let model_window = *fixture.model().windows().keys().next().expect("a window");
    let rect = crate::protocol::xdg_shell::rect_of(&fixture, model_window);
    assert_eq!(
        (rect.origin.x, rect.origin.y, rect.size.w, rect.size.h),
        (0, 0, 1920, 1080),
        "it is placed over the bar's strip"
    );
    crate::protocol::xdg_shell::settle(&mut fixture);
    fixture
        .input()
        .pointer_to(mouse, LogicalPoint { x: 420, y: 20 });
    // The window was under the pointer (the output's centre) already, so it is a motion that
    // arrives, at the window's own coordinates: the window starts at the output's origin.
    let motions: Vec<_> = events_of(fixture.client(app), "wl_pointer#")
        .into_iter()
        .filter(|(event, _)| event == "motion")
        .collect();
    assert_eq!(
        motions.last().map(|(_, args)| args[1..].to_vec()),
        Some(vec!["420.000".to_owned(), "20.000".to_owned()]),
        "the pointer in the bar's old strip is over the window: {motions:?}"
    );
}

/// cas-wave "Real-run checks: Layers": a panel whose buffer carries a shadow margin around its
/// body and whose input region is the body alone. A pointer in the margin, over a window below,
/// is the window's (it enters the window, not the panel), a pointer in the body is the panel's,
/// and the margin still draws over the window.
#[test]
fn a_panels_shadow_margin_takes_no_input_and_still_draws() {
    // The panel's buffer is 340 x 240 at the output's corner; its body is the inner 300 x 200.
    let margin = (20, 20, 300, 200);
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let mouse = fixture.input().plug(
        DeviceClass::Pointer,
        TestOrigin::Physical(DeviceClass::Pointer),
    );
    let (app, shell) = (
        fixture.connect(ConnectionRole::Public),
        fixture.connect(ConnectionRole::Public),
    );
    fixture.pump();
    let _pointers = [app, shell].map(|id| Pointer::bind(fixture.client(id)).expect("a pointer"));
    let base = WmBase::bind(fixture.client(app), 7);
    let _window = map_toplevel(&mut fixture, app, &base, (600, 500));
    let window = windows(&fixture)[0];
    fixture.place_window_by_hand(window, LogicalPoint { x: 0, y: 0 });
    settle(&mut fixture);
    let ask = LayerAsk {
        level: Layer::Top,
        namespace: "panel",
        anchor: Anchor::Top | Anchor::Left,
        exclusive_zone: 0,
        keyboard: KeyboardInteractivity::None,
        size: (340, 240),
    };
    let panel = map(&mut fixture, shell, &ask);
    let compositor = Compositor::bind(fixture.client(shell), 4);
    let region = compositor
        .compositor
        .create_region(&fixture.client(shell).qh(), H1Data);
    region.add(margin.0, margin.1, margin.2, margin.3);
    panel.surface().set_input_region(Some(&region));
    region.destroy();
    panel.surface().commit();
    fixture.pump();

    let clients = [app, shell];
    // name, where the pointer goes, (window enters, panel enters)
    let moves = [
        ("in the margin: the window below", (5, 150), [1, 0]),
        ("in the body: the panel", (100, 100), [0, 1]),
        ("back in the margin: the window", (330, 230), [1, 0]),
    ];
    for (name, at, entered) in moves {
        assert_eq!(
            pointer_enters_at(&mut fixture, mouse, at, &clients),
            entered,
            "{name}"
        );
    }
    // Out of the way of the cursor, the margin is the panel's pixels over the window's.
    fixture
        .input()
        .pointer_to(mouse, LogicalPoint { x: 1000, y: 800 });
    let output = OutputName::parse("HEADLESS-1").expect("a name");
    fixture.advance(std::time::Duration::from_secs(1));
    fixture.frames(&output, FrameCount(2));
    let shot = fixture.shot(ShotTarget::Output(output));
    assert_eq!(
        shot.get_pixel(5, 150).0,
        [0x20, 0x20, 0x20, 0xff],
        "the margin is drawn over the window"
    );
}

/// CASC-03: a client may hold `MAX_LAYERS` layer surfaces; the next one cuts it with
/// `wl_display.no_memory` (code 2).
#[test]
fn a_client_that_makes_layer_surfaces_without_end_is_cut() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let id = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let mut made = Vec::new();
    for _ in 0..casement_wayland::limits::MAX_LAYERS {
        made.push(LayerSurface::create(fixture.client(id), &wallpaper()).expect("a layer"));
    }
    fixture.pump();
    assert!(
        fixture.client(id).protocol_error().is_none(),
        "up to the cap is allowed"
    );
    made.push(LayerSurface::create(fixture.client(id), &wallpaper()).expect("a layer"));
    fixture.pump();
    // The cut closes the connection (the system backend sends no error first).
    assert!(
        !fixture.client(id).is_connected(),
        "the client past the cap is cut"
    );
}
