//! Where things are drawn and hit: a toplevel's window geometry (`xdg_surface.set_window_geometry`)
//! and a layer surface's configured box. What the renderer draws, what the pointer enters and
//! what a client is told must agree.

use super::xdg_shell::{boot, pointer, rect_of, settle, windows};
use casement_core::geometry::{LogicalPoint, PhysicalSize};
use casement_core::window::WindowState;
use casement_testclient::seat::pointer::Pointer;
use casement_testclient::surface::layer::{
    Anchor, KeyboardInteractivity, Layer, LayerAsk, LayerSurface,
};
use casement_testclient::surface::toplevel::{State, Toplevel, WmBase};
use casement_testclient::{Pattern, TestClient, shm_buffer};
use casement_testkit::{ClientId, Fixture, ShotTarget};
use smithay::reexports::wayland_server::Resource as _;
use wayland_client::Proxy as _;

/// The surface-local position of the newest `wl_pointer.enter` or `motion` the client was sent.
fn last_enter(client: &TestClient) -> Option<(f64, f64)> {
    let lines = &client.transcript().0;
    let line = lines.iter().rev().find(|line| {
        line.object.starts_with("wl_pointer#") && matches!(line.event.as_str(), "enter" | "motion")
    })?;
    // enter: serial, surface, x, y; motion: time, x, y.
    let first = if line.event == "enter" { 2 } else { 1 };
    let coord = |at: usize| line.args.get(first + at)?.parse::<f64>().ok();
    Some((coord(0)?, coord(1)?))
}

/// A toplevel that sets its window geometry to `geometry` (x, y, w, h) before it maps, with a
/// buffer of `size` that encodes each pixel's coordinates.
fn map_with_geometry(
    fixture: &mut Fixture,
    id: ClientId,
    base: &WmBase,
    size: (u32, u32),
    geometry: (i32, i32, i32, i32),
) -> Toplevel {
    let top = base.toplevel(fixture.client(id));
    top.commit_bare();
    fixture.pump();
    assert!(top.ack_latest().is_some(), "the initial configure arrives");
    top.xdg_surface
        .set_window_geometry(geometry.0, geometry.1, geometry.2, geometry.3);
    let buffer = shm_buffer(
        fixture.client(id),
        PhysicalSize {
            w: size.0,
            h: size.1,
        },
        Pattern::CoordinateEncoding,
    );
    top.show(&buffer);
    fixture.pump();
    settle(fixture);
    top
}

/// The window's place is its geometry's top-left: the pointer a few pixels inside the window
/// is that far inside the geometry, which is the geometry's offset further into the surface.
#[test]
fn the_pointer_is_local_to_the_surface_at_the_geometry_offset() {
    let (mut fixture, id) = boot();
    let _pointer = Pointer::bind(fixture.client(id)).expect("a pointer");
    let mouse = pointer(&mut fixture);
    let base = WmBase::bind(fixture.client(id), 6);
    let _top = map_with_geometry(&mut fixture, id, &base, (400, 300), (20, 30, 360, 240));
    let window = windows(&fixture)[0];
    let rect = rect_of(&fixture, window);
    assert_eq!(
        (rect.size.w, rect.size.h),
        (360, 240),
        "the window is its geometry's size"
    );
    fixture.input().pointer_to(
        mouse,
        LogicalPoint {
            x: rect.origin.x + 5,
            y: rect.origin.y + 7,
        },
    );
    assert_eq!(
        last_enter(fixture.client(id)),
        Some((25.0, 37.0)),
        "5 and 7 into the geometry is 25 and 37 into the surface"
    );
}

/// The buffer is drawn offset so that its geometry origin lands at the window's place: the
/// window's top edge is the surface's row 30 from column 20 (the corner itself is rounded, as a
/// window that says nothing of its frame is).
#[test]
fn the_buffer_is_drawn_with_its_geometry_origin_at_the_window() {
    let (mut fixture, id) = boot();
    let base = WmBase::bind(fixture.client(id), 6);
    let _top = map_with_geometry(&mut fixture, id, &base, (400, 300), (20, 30, 360, 240));
    let window = windows(&fixture)[0];
    let shot = fixture.shot(ShotTarget::Window(window));
    // Past the corner's curve on the top edge.
    let first = shot.get_pixel(16, 0).0;
    assert_eq!(
        (first[0], first[1]),
        (36, 30),
        "the pixel 16 along the window's top edge is the surface's (36, 30): {first:?}"
    );
    let inner = shot.get_pixel(10, 10).0;
    assert_eq!((inner[0], inner[1]), (30, 40));
}

/// A layer ask on the primary output with nothing else.
fn ask(anchor: Anchor, size: (u32, u32)) -> LayerAsk {
    LayerAsk {
        level: Layer::Top,
        namespace: "test",
        anchor,
        exclusive_zone: 0,
        keyboard: KeyboardInteractivity::None,
        size,
    }
}

/// What the first configure says, for a layer asking `ask`.
fn configured(ask: &LayerAsk) -> (u32, u32) {
    let (mut fixture, id) = boot();
    let layer = LayerSurface::create(fixture.client(id), ask).expect("a layer surface");
    fixture.pump();
    let told = layer.configure().expect("the initial configure");
    (told.width, told.height)
}

/// A size the surface asked for is the size it is configured to, on an axis it asked for one,
/// whatever it is anchored to (wlr-layer-shell `set_size`: only a zero is the compositor's to
/// assign).
#[test]
fn a_layer_is_configured_to_the_size_it_asked_for() {
    let every_edge = Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right;
    // name, ask, configure
    let cases = [
        ("unanchored", ask(Anchor::empty(), (123, 546)), (123, 546)),
        ("every edge, sized", ask(every_edge, (321, 218)), (321, 218)),
        ("every edge, unsized", ask(every_edge, (0, 0)), (1920, 1080)),
        (
            "a bar: the width is the room's, the height its own",
            ask(Anchor::Top | Anchor::Left | Anchor::Right, (0, 28)),
            (1920, 28),
        ),
    ];
    for (case, ask, want) in cases {
        assert_eq!(configured(&ask), want, "{case}");
    }
}

/// A surface smaller than the box it was configured to sits at the box's corner, and the box,
/// not the buffer, is where it is placed: against the bottom right here.
#[test]
fn a_layer_is_placed_by_its_configured_box_not_its_buffer() {
    let (mut fixture, id) = boot();
    let mouse = pointer(&mut fixture);
    let _pointer = Pointer::bind(fixture.client(id)).expect("a pointer");
    let layer = LayerSurface::create(
        fixture.client(id),
        &ask(Anchor::Bottom | Anchor::Right, (200, 300)),
    )
    .expect("a layer surface");
    fixture.pump();
    assert!(layer.ack(fixture.client(id)));
    let buffer = shm_buffer(
        fixture.client(id),
        PhysicalSize { w: 52, h: 74 },
        Pattern::Solid(0xff20_2020),
    );
    layer.show(fixture.client(id), &buffer);
    fixture.pump();
    // The 200x300 box is at (1720, 780): ten pixels into it is ten into the surface.
    fixture
        .input()
        .pointer_to(mouse, LogicalPoint { x: 1730, y: 790 });
    assert_eq!(last_enter(fixture.client(id)), Some((10.0, 10.0)));
}

/// A toplevel that asks to be maximised before it maps is configured maximised, to the room
/// the layers leave, in its initial configure (a client that maps from the size it is told
/// would otherwise never map), and the model keeps it filled.
#[test]
fn a_toplevel_that_asks_to_open_maximised_is_configured_so_at_once() {
    let (mut fixture, id) = boot();
    let shell = fixture.connect(casement_core::policy::ConnectionRole::Public);
    fixture.pump();
    let bar = LayerAsk {
        exclusive_zone: 28,
        ..ask(Anchor::Top | Anchor::Left | Anchor::Right, (0, 28))
    };
    let bar_surface = LayerSurface::create(fixture.client(shell), &bar).expect("a bar");
    fixture.pump();
    assert!(bar_surface.ack(fixture.client(shell)));
    let buffer = shm_buffer(
        fixture.client(shell),
        PhysicalSize { w: 1920, h: 28 },
        Pattern::Solid(0xff20_2020),
    );
    bar_surface.show(fixture.client(shell), &buffer);
    fixture.pump();
    let base = WmBase::bind(fixture.client(id), 6);
    let top = base.toplevel(fixture.client(id));
    top.set_maximized();
    top.commit_bare();
    fixture.pump();
    let told = top
        .configures()
        .last()
        .expect("the initial configure")
        .clone();
    assert!(told.states.contains(&State::Maximized), "{told:?}");
    assert_eq!(
        told.size,
        (1920, 1080 - 28),
        "the room under the bar: {told:?}"
    );
    assert!(top.ack_latest().is_some());
    let buffer = shm_buffer(
        fixture.client(id),
        PhysicalSize { w: 1920, h: 1052 },
        Pattern::Solid(0xff80_8080),
    );
    top.show(&buffer);
    fixture.pump();
    let model = fixture.model();
    let window = model.windows().values().next().expect("the window mapped");
    assert!(
        matches!(window.state, WindowState::Filled { .. }),
        "the model heard the ask once it mapped: {:?}",
        window.state
    );
}

/// Full screen before the map is configured the same way, to the whole output.
#[test]
fn a_toplevel_that_asks_to_open_full_screen_is_configured_to_the_whole_output() {
    let (mut fixture, id) = boot();
    let base = WmBase::bind(fixture.client(id), 6);
    let top = base.toplevel(fixture.client(id));
    top.set_fullscreen();
    top.commit_bare();
    fixture.pump();
    let told = top
        .configures()
        .last()
        .expect("the initial configure")
        .clone();
    assert!(told.states.contains(&State::Fullscreen), "{told:?}");
    assert_eq!(told.size, (1920, 1080));
}

/// A layer surface put by hand is hit where it was put (what wlcs' `move_surface_to` asks of a
/// layer), and the surfaces under it keep their own places.
#[test]
fn a_layer_put_by_hand_is_hit_where_it_was_put() {
    let (mut fixture, id) = boot();
    let mouse = pointer(&mut fixture);
    let _pointer = Pointer::bind(fixture.client(id)).expect("a pointer");
    let layer = LayerSurface::create(fixture.client(id), &ask(Anchor::empty(), (200, 100)))
        .expect("a layer surface");
    fixture.pump();
    assert!(layer.ack(fixture.client(id)));
    let buffer = shm_buffer(
        fixture.client(id),
        PhysicalSize { w: 200, h: 100 },
        Pattern::Solid(0xff20_2020),
    );
    layer.show(fixture.client(id), &buffer);
    fixture.pump();
    let object = layer.surface().id().protocol_id();
    let key = fixture.layer_of(id, object).expect("a layer surface");
    fixture.place_layer(key, LogicalPoint { x: 40, y: 50 });
    fixture
        .input()
        .pointer_to(mouse, LogicalPoint { x: 45, y: 58 });
    assert_eq!(last_enter(fixture.client(id)), Some((5.0, 8.0)));
}

/// Moving a surface to a higher layer puts it above the one it shared a place with.
#[test]
fn a_surface_moved_to_a_higher_layer_is_hit_above_the_other() {
    let (mut fixture, id) = boot();
    let mouse = pointer(&mut fixture);
    let _pointer = Pointer::bind(fixture.client(id)).expect("a pointer");
    let mut map = |level: Layer| {
        let asked = LayerAsk {
            level,
            ..ask(Anchor::empty(), (200, 100))
        };
        let layer = LayerSurface::create(fixture.client(id), &asked).expect("a layer surface");
        fixture.pump();
        assert!(layer.ack(fixture.client(id)));
        let buffer = shm_buffer(
            fixture.client(id),
            PhysicalSize { w: 200, h: 100 },
            Pattern::Solid(0xff20_2020),
        );
        layer.show(fixture.client(id), &buffer);
        fixture.pump();
        layer
    };
    let lower = map(Layer::Bottom);
    let upper = map(Layer::Top);
    let centre = LogicalPoint { x: 960, y: 540 };
    fixture.input().pointer_to(mouse, centre);
    let on_top = |fixture: &Fixture| {
        fixture
            .pointer_focus()
            .map(|surface| surface.id().protocol_id())
    };
    let (lower_id, upper_id) = (
        lower.surface().id().protocol_id(),
        upper.surface().id().protocol_id(),
    );
    assert_eq!(on_top(&fixture), Some(upper_id));
    lower.set_layer(fixture.client(id), Layer::Overlay);
    fixture.pump();
    fixture.refocus_pointer();
    assert_eq!(on_top(&fixture), Some(lower_id), "it is on the overlay now");
}

/// A popup of a layer surface is kept inside the output by its positioner's adjustments, as a
/// window's popup is: a menu that would hang off the right edge slides back.
#[test]
fn a_layer_popup_is_kept_inside_the_output() {
    use casement_testclient::surface::popup::{Popup, PopupSpec};
    let (mut fixture, id) = boot();
    let layer = LayerSurface::create(
        fixture.client(id),
        &ask(Anchor::Bottom | Anchor::Right, (200, 100)),
    )
    .expect("a layer surface");
    fixture.pump();
    assert!(layer.ack(fixture.client(id)));
    let buffer = shm_buffer(
        fixture.client(id),
        PhysicalSize { w: 200, h: 100 },
        Pattern::Solid(0xff20_2020),
    );
    layer.show(fixture.client(id), &buffer);
    fixture.pump();
    let base = WmBase::bind(fixture.client(id), 7);
    let surface = fixture.client(id).create_surface().expect("a surface");
    // At (150, 80) of a layer whose box starts at x 1720, 100 wide, the menu ends at 1970 > 1920.
    let popup = Popup::of_layer(
        fixture.client(id),
        &base,
        surface,
        PopupSpec::menu_at((150, 80)),
    );
    layer.adopt(&popup.popup);
    popup.surface.commit();
    fixture.pump();
    assert_eq!(
        popup.geometry(),
        Some((100, 50, 100, 50)),
        "slid left by the 50 pixels it overhung on the right and up by the 30 on the bottom"
    );
}
