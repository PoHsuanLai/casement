//! `wlr_data_control` (P0-H5).

use casement_core::geometry::PhysicalSize;
use casement_core::policy::ConnectionRole;
use casement_testclient::seat::keyboard::Keyboard;
use casement_testclient::selection::data_control::wlr::WlrControl;
use casement_testclient::selection::data_device::DataDevice;
use casement_testclient::surface::layer::{
    Anchor, KeyboardInteractivity, Layer, LayerAsk, LayerSurface,
};
use casement_testclient::{Pattern, shm_buffer};
use casement_testkit::ClientId;
use casement_testkit::{Fixture, FixtureSpec};

/// A clipboard manager is told of every new selection.
#[test]
fn a_manager_sees_selection_changes() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let owner = fixture.connect(ConnectionRole::Shell);
    let manager = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let keyboard = Keyboard::bind(fixture.client(owner)).expect("keyboard");
    let device = DataDevice::bind(fixture.client(owner), keyboard.seat(), None).expect("device");
    let manager_keyboard = Keyboard::bind(fixture.client(manager)).expect("keyboard");
    let _control =
        WlrControl::bind(fixture.client(manager), manager_keyboard.seat()).expect("manager");
    let _layer = map(&mut fixture, owner, &focused(Layer::Top, "owner"));
    device.copy(fixture.client(owner), "text/plain", 0);
    fixture.pump();
    device.copy(fixture.client(owner), "text/uri-list", 0);
    fixture.pump();
    let selections = fixture
        .client(manager)
        .transcript()
        .0
        .iter()
        .filter(|line| line.event == "selection")
        .count();
    assert!(
        selections >= 2,
        "the manager was told of both changes, saw {selections}"
    );
}
/// A layer of 400x300 at the centre of the output, at `level`, taking the keyboard exclusively.
fn focused(level: Layer, namespace: &'static str) -> LayerAsk {
    LayerAsk {
        level,
        namespace,
        anchor: Anchor::empty(),
        exclusive_zone: 0,
        keyboard: KeyboardInteractivity::Exclusive,
        size: (400, 300),
    }
}

/// Maps `ask` the way a shell does: create, wait for the configure, acknowledge it and attach
/// a buffer of the configured size.
fn map(fixture: &mut Fixture, id: ClientId, ask: &LayerAsk) -> LayerSurface {
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
