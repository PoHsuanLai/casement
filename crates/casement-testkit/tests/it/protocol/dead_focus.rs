//! A surface that dies stops being the seat's focus: destroying a toplevel, layer surface or
//! popup under the pointer must not send it `wl_pointer.leave` (the dead object makes
//! wayland-backend panic), and a touch point on a surface that dies is released.

use super::xdg_shell::{boot, map_toplevel, pointer, rect_of, settle, windows};
use casement_core::geometry::{LogicalPoint, PhysicalSize};
use casement_core::input::{DeviceClass, TouchStep};
use casement_testclient::seat::pointer::Pointer;
use casement_testclient::seat::touch::Touch;
use casement_testclient::surface::layer::{
    Anchor, KeyboardInteractivity, Layer, LayerAsk, LayerSurface,
};
use casement_testclient::surface::popup::{Popup, PopupSpec};
use casement_testclient::surface::toplevel::{Toplevel, WmBase};
use casement_testclient::{Pattern, TestClient, shm_buffer};
use casement_testkit::{ClientId, Fixture, TestOrigin};

/// How many events named `event` the objects of interface `object` received.
fn count(client: &TestClient, object: &str, event: &str) -> usize {
    client
        .transcript()
        .0
        .iter()
        .filter(|line| line.object.starts_with(object) && line.event == event)
        .count()
}

/// Destroys `top` surface first, as a client may: its role objects outlive it for a moment, and
/// the compositor hears of the toplevel's end with the surface already gone.
fn destroy_surface_first(fixture: &mut Fixture, id: ClientId, top: &Toplevel) {
    top.surface.destroy();
    fixture.pump();
    assert!(
        fixture.pointer_focus().is_none(),
        "a surface that is gone holds no pointer"
    );
    top.destroy_roles();
    fixture.pump();
    fixture.pump();
    let _ = id;
}

/// Moves the pointer to `at` and lets the client hear it.
fn point_at(fixture: &mut Fixture, id: ClientId, at: LogicalPoint) {
    let mouse = pointer(fixture);
    fixture.input().pointer_to(mouse, at);
    fixture.pump();
    assert!(
        count(fixture.client(id), "wl_pointer#", "enter") > 0,
        "the pointer entered the surface"
    );
}

#[test]
fn a_toplevel_destroyed_under_the_pointer_gets_no_leave() {
    let (mut fixture, id) = boot();
    let _pointer = Pointer::bind(fixture.client(id)).expect("pointer");
    let base = WmBase::bind(fixture.client(id), 7);
    let top = map_toplevel(&mut fixture, id, &base, (400, 300));
    settle(&mut fixture);
    let window = windows(&fixture)[0];
    let at = rect_of(&fixture, window).centre();
    point_at(&mut fixture, id, at);
    let leaves = count(fixture.client(id), "wl_pointer#", "leave");
    destroy_surface_first(&mut fixture, id, &top);
    assert_eq!(count(fixture.client(id), "wl_pointer#", "leave"), leaves);
    assert!(
        fixture.pointer_focus().is_none(),
        "the dead surface is no longer the focus"
    );
    // The pointer goes on working: it moves, and a new window under it gets the enter.
    let mouse = pointer(&mut fixture);
    fixture.input().pointer_to(
        mouse,
        LogicalPoint {
            x: at.x + 5,
            y: at.y,
        },
    );
    fixture.pump();
}

#[test]
fn a_layer_surface_destroyed_under_the_pointer_gets_no_leave() {
    let (mut fixture, id) = boot();
    let _pointer = Pointer::bind(fixture.client(id)).expect("pointer");
    let ask = LayerAsk {
        level: Layer::Top,
        namespace: "bar",
        anchor: Anchor::Top | Anchor::Left | Anchor::Right,
        exclusive_zone: 30,
        keyboard: KeyboardInteractivity::None,
        size: (0, 30),
    };
    let layer = LayerSurface::create(fixture.client(id), &ask).expect("a layer surface");
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
    point_at(&mut fixture, id, LogicalPoint { x: 110, y: 10 });
    let leaves = count(fixture.client(id), "wl_pointer#", "leave");
    layer.destroy_surface_first(fixture.client(id));
    fixture.pump();
    fixture.pump();
    assert_eq!(count(fixture.client(id), "wl_pointer#", "leave"), leaves);
    assert!(
        fixture.pointer_focus().is_none(),
        "the dead surface is no longer the focus"
    );
}

/// The client destroys the popup's objects newest first, so the pointer is told it left the popup
/// while the `wl_surface` still lives (once, at the `xdg_popup`'s destruction: P0-popups, W8), and
/// nothing is sent to the surface after it is gone.
#[test]
fn a_popup_destroyed_under_the_pointer_is_left_once_and_nothing_follows() {
    let (mut fixture, id) = boot();
    let _pointer = Pointer::bind(fixture.client(id)).expect("pointer");
    let base = WmBase::bind(fixture.client(id), 7);
    let top = map_toplevel(&mut fixture, id, &base, (400, 300));
    settle(&mut fixture);
    let window = windows(&fixture)[0];
    let origin = rect_of(&fixture, window).origin;
    let surface = fixture.client(id).create_surface().expect("a surface");
    let spec = PopupSpec {
        size: (100, 50),
        anchor_rect: (20, 20, 1, 1),
        ..PopupSpec::menu_at((0, 0))
    };
    let popup = Popup::of(fixture.client(id), &base, &top, surface, spec);
    popup.surface.commit();
    fixture.pump();
    assert!(popup.ack_latest().is_some(), "the popup was configured");
    let buffer = shm_buffer(
        fixture.client(id),
        PhysicalSize { w: 100, h: 50 },
        Pattern::Solid(0xff40_4040),
    );
    popup.show(&buffer);
    fixture.pump();
    let (x, y, w, h) = popup.geometry().expect("the popup's place");
    let at = LogicalPoint {
        x: origin.x + x + w / 2,
        y: origin.y + y + h / 2,
    };
    point_at(&mut fixture, id, at);
    let leaves = count(fixture.client(id), "wl_pointer#", "leave");
    popup.destroy();
    fixture.pump();
    fixture.pump();
    assert_eq!(
        count(fixture.client(id), "wl_pointer#", "leave"),
        leaves + 1,
        "left once"
    );
    assert_eq!(fixture.client(id).protocol_error(), None);
}

#[test]
fn touch_on_a_surface_that_dies_is_released() {
    let (mut fixture, id) = boot();
    let _touch = Touch::bind(fixture.client(id)).expect("touch");
    let screen = fixture
        .input()
        .plug(DeviceClass::Touch, TestOrigin::Physical(DeviceClass::Touch));
    let base = WmBase::bind(fixture.client(id), 7);
    let top = map_toplevel(&mut fixture, id, &base, (400, 300));
    settle(&mut fixture);
    let window = windows(&fixture)[0];
    let at = rect_of(&fixture, window).centre();
    fixture
        .input()
        .touch(screen, TouchStep::Down { slot: 0, at });
    fixture.input().touch(screen, TouchStep::Frame);
    fixture.pump();
    assert_eq!(count(fixture.client(id), "wl_touch#", "down"), 1);
    destroy_surface_first(&mut fixture, id, &top);
    assert_eq!(
        count(fixture.client(id), "wl_touch#", "up"),
        1,
        "the touch point was released once"
    );
}

#[test]
fn a_client_that_goes_away_under_the_pointer_takes_no_leave_with_it() {
    let (mut fixture, id) = boot();
    let base = WmBase::bind(fixture.client(id), 7);
    let _top = map_toplevel(&mut fixture, id, &base, (400, 300));
    // The pointer is bound after the window: libwayland destroys a hung-up client's objects
    // in the order of their ids, so the surface dies while its pointer still stands.
    let _pointer = Pointer::bind(fixture.client(id)).expect("pointer");
    settle(&mut fixture);
    let window = windows(&fixture)[0];
    let at = rect_of(&fixture, window).centre();
    point_at(&mut fixture, id, at);
    fixture.disconnect(id);
    fixture.pump();
    fixture.pump();
    assert!(
        fixture.pointer_focus().is_none(),
        "the dead surface is no longer the focus"
    );
    assert!(
        windows(&fixture).is_empty(),
        "the window went with its client"
    );
    // The seat goes on working: a later client gets the pointer.
    let mouse = pointer(&mut fixture);
    fixture.input().pointer_to(
        mouse,
        LogicalPoint {
            x: at.x + 5,
            y: at.y,
        },
    );
    fixture.pump();
}
