//! `zwlr_layer_surface_v1.set_keyboard_interactivity(on_demand)`: the surface takes the keyboard
//! when it maps with no window holding it, and when it is pressed; a window that is pressed or
//! keyed takes it back.

use super::xdg_shell::{boot, click, map_toplevel, pointer, settle, windows};
use casement_core::geometry::{LogicalPoint, PhysicalSize};
use casement_core::input::{ButtonCode, Edge};
use casement_testclient::seat::keyboard::Keyboard;
use casement_testclient::surface::layer::{
    Anchor, KeyboardInteractivity, Layer, LayerAsk, LayerSurface,
};
use casement_testclient::surface::toplevel::WmBase;
use casement_testclient::{Pattern, TestClient, shm_buffer};
use casement_testkit::{ClientId, Fixture};
use wayland_client::Proxy;
use wayland_client::protocol::wl_surface::WlSurface;

/// A 200 x 100 on-demand layer surface in the middle of the output.
fn on_demand() -> LayerAsk {
    LayerAsk {
        level: Layer::Top,
        namespace: "launcher",
        anchor: Anchor::empty(),
        exclusive_zone: 0,
        keyboard: KeyboardInteractivity::OnDemand,
        size: (200, 100),
    }
}

/// Maps `ask`: create, ack, attach a buffer of its size.
fn map(fixture: &mut Fixture, id: ClientId, ask: &LayerAsk) -> LayerSurface {
    let layer = LayerSurface::create(fixture.client(id), ask).expect("a layer surface");
    fixture.pump();
    assert!(layer.ack(fixture.client(id)));
    let buffer = shm_buffer(
        fixture.client(id),
        PhysicalSize {
            w: ask.size.0,
            h: ask.size.1,
        },
        Pattern::Solid(0xff20_2020),
    );
    layer.show(fixture.client(id), &buffer);
    fixture.pump();
    layer
}

/// The surface the client's keyboard last entered and has not left, as it reads in a transcript.
fn keyboard_focus(client: &TestClient) -> Option<String> {
    client
        .transcript()
        .0
        .iter()
        .filter(|line| line.object.starts_with("wl_keyboard#"))
        .fold(None, |held, line| match line.event.as_str() {
            "enter" => line.args.get(1).cloned(),
            "leave" => None,
            _ => held,
        })
}

fn named(surface: &WlSurface) -> String {
    format!("wl_surface#{}", surface.id().protocol_id())
}

/// With no window holding the keyboard, an on-demand layer that maps takes it.
#[test]
fn an_on_demand_layer_takes_the_keyboard_when_nothing_holds_it() {
    let (mut fixture, id) = boot();
    let _keyboard = Keyboard::bind(fixture.client(id)).expect("a keyboard");
    let layer = map(&mut fixture, id, &on_demand());
    assert_eq!(
        keyboard_focus(fixture.client(id)),
        Some(named(layer.surface()))
    );
}

/// A window that holds the keyboard keeps it when an on-demand layer maps; a press on the layer
/// takes it, and a press on the window gives it back.
#[test]
fn a_press_moves_the_keyboard_between_an_on_demand_layer_and_a_window() {
    let (mut fixture, id) = boot();
    let mouse = pointer(&mut fixture);
    let _keyboard = Keyboard::bind(fixture.client(id)).expect("a keyboard");
    let base = WmBase::bind(fixture.client(id), 6);
    let top = map_toplevel(&mut fixture, id, &base, (400, 300));
    settle(&mut fixture);
    let window = windows(&fixture)[0];
    // A window that maps takes no focus (A7): a click activates it.
    click(&mut fixture, mouse, window);
    assert_eq!(
        keyboard_focus(fixture.client(id)),
        Some(named(&top.surface))
    );
    let layer = map(&mut fixture, id, &on_demand());
    assert_eq!(
        keyboard_focus(fixture.client(id)),
        Some(named(&top.surface)),
        "a layer that maps over a window that has the keyboard does not take it"
    );
    // The layer is in the middle of the output, above the window.
    fixture
        .input()
        .pointer_to(mouse, LogicalPoint { x: 960, y: 540 });
    fixture
        .input()
        .button(mouse, ButtonCode(0x110), Edge::Press);
    fixture
        .input()
        .button(mouse, ButtonCode(0x110), Edge::Release);
    fixture.pump();
    assert_eq!(
        keyboard_focus(fixture.client(id)),
        Some(named(layer.surface())),
        "a press on the layer gives it the keyboard"
    );
    // The window's corner is outside the layer.
    let rect = super::xdg_shell::rect_of(&fixture, window);
    fixture.input().pointer_to(
        mouse,
        LogicalPoint {
            x: rect.origin.x + 4,
            y: rect.origin.y + 4,
        },
    );
    fixture
        .input()
        .button(mouse, ButtonCode(0x110), Edge::Press);
    fixture
        .input()
        .button(mouse, ButtonCode(0x110), Edge::Release);
    fixture.pump();
    assert_eq!(
        keyboard_focus(fixture.client(id)),
        Some(named(&top.surface)),
        "a press on the window takes the keyboard back"
    );
}
