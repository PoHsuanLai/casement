//! `primary_selection` (P0-H5).

use crate::protocol::data_device::{focused, map, saw};
use casement_core::policy::ConnectionRole;
use casement_testclient::seat::keyboard::Keyboard;
use casement_testclient::selection::primary::PrimaryDevice;
use casement_testclient::surface::layer::Layer;
use casement_testkit::{Fixture, FixtureSpec};

/// The primary selection is offered to the focused client.
#[test]
fn primary_selection_follows_focus() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let a = fixture.connect(ConnectionRole::Public);
    let b = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let keyboard_a = Keyboard::bind(fixture.client(a)).expect("keyboard");
    let keyboard_b = Keyboard::bind(fixture.client(b)).expect("keyboard");
    let device_a = PrimaryDevice::bind(fixture.client(a), keyboard_a.seat()).expect("device");
    let _device_b = PrimaryDevice::bind(fixture.client(b), keyboard_b.seat()).expect("device");
    // A holds the keyboard and selects: the selection is offered to A (the focused client)
    // and to nobody else.
    let _layer_a = map(&mut fixture, a, &focused(Layer::Top, "a"));
    device_a.select(fixture.client(a), "text/plain", 0);
    fixture.pump();
    assert!(
        saw(
            fixture.client(a),
            "zwp_primary_selection_device_v1#",
            "selection"
        ),
        "the focused client hears its own selection"
    );
    assert!(
        !saw(
            fixture.client(b),
            "zwp_primary_selection_device_v1#",
            "selection"
        ),
        "an unfocused client is offered nothing"
    );
    // B takes the keyboard: the selection is offered to it.
    let _layer_b = map(&mut fixture, b, &focused(Layer::Overlay, "b"));
    assert!(saw(
        fixture.client(b),
        "zwp_primary_selection_device_v1#",
        "data_offer"
    ));
    assert!(saw(
        fixture.client(b),
        "zwp_primary_selection_device_v1#",
        "selection"
    ));
}
