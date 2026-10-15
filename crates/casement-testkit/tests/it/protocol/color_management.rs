//! `color_management` (P3-CM): the parametric subset, an output's description, and a surface's tag.

use casement_core::policy::ConnectionRole;
use casement_testclient::misc::color_management::{
    ColorManagement, Primaries, RenderIntent, TransferFunction, describe, information,
};
use casement_testclient::surface::subsurface::{Compositor, bind_outputs, seen};
use casement_testkit::{ClientId, Fixture, FixtureSpec};

fn boot() -> (Fixture, ClientId) {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let id = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    (fixture, id)
}

/// The (interface, code) of the protocol error the compositor killed `id` with.
fn fault(fixture: &mut Fixture, id: ClientId) -> (String, u32) {
    let error = fixture
        .client(id)
        .protocol_error()
        .expect("the compositor posted an error");
    (error.interface, error.code)
}

/// The names of the events `object` received, in order.
fn names(fixture: &mut Fixture, id: ClientId, object: &impl wayland_client::Proxy) -> Vec<String> {
    seen(fixture.client(id), object)
        .into_iter()
        .map(|(event, _)| event)
        .collect()
}

/// Binding announces the one intent, the two features, three curves and two named primaries, then
/// `done`.
#[test]
fn the_manager_announces_what_it_serves() {
    let (mut fixture, id) = boot();
    let manager = ColorManagement::bind(fixture.client(id), 2);
    fixture.pump();
    let events = names(&mut fixture, id, &manager.manager);
    let count = |name: &str| events.iter().filter(|event| *event == name).count();
    assert_eq!(count("supported_intent"), 1);
    assert_eq!(count("supported_feature"), 2);
    assert_eq!(count("supported_tf_named"), 3);
    assert_eq!(count("supported_primaries_named"), 2);
    assert_eq!(events.last().map(String::as_str), Some("done"));
}

/// Named primaries and a named curve make a ready description with an identity (version 2).
#[test]
fn a_parametric_description_becomes_ready() {
    let (mut fixture, id) = boot();
    let manager = ColorManagement::bind(fixture.client(id), 2);
    let creator = manager.creator(fixture.client(id));
    let description = describe(
        fixture.client(id),
        creator,
        Primaries::DisplayP3,
        TransferFunction::Gamma22,
    );
    fixture.pump();
    assert!(fixture.client(id).protocol_error().is_none());
    assert_eq!(names(&mut fixture, id, &description), ["ready2"]);
}

/// A creator with no curve cannot create: wp_image_description_creator_params_v1.incomplete_set (0).
#[test]
fn creating_without_a_curve_is_a_protocol_error() {
    let (mut fixture, id) = boot();
    let manager = ColorManagement::bind(fixture.client(id), 2);
    let creator = manager.creator(fixture.client(id));
    creator.set_primaries_named(Primaries::Srgb);
    let _description = creator.create(
        &fixture.client(id).qh(),
        casement_testclient::surface::subsurface::H1Data,
    );
    fixture.pump();
    assert_eq!(
        fault(&mut fixture, id),
        ("wp_image_description_creator_params_v1".to_owned(), 0)
    );
}

/// ICC profiles are not served: wp_color_manager_v1.unsupported_feature (0).
#[test]
fn an_icc_creator_is_unsupported() {
    let (mut fixture, id) = boot();
    let manager = ColorManagement::bind(fixture.client(id), 2);
    let _creator = manager.icc_creator(fixture.client(id));
    fixture.pump();
    assert_eq!(
        fault(&mut fixture, id),
        ("wp_color_manager_v1".to_owned(), 0)
    );
}

/// An output describes itself, and its description allows `get_information`, which ends in `done`
/// and starts with the primaries (an sRGB panel: red is 0.640, 0.330).
#[test]
fn an_output_describes_its_colours() {
    let (mut fixture, id) = boot();
    let outputs = bind_outputs(fixture.client(id), 4, false).expect("outputs");
    let manager = ColorManagement::bind(fixture.client(id), 2);
    let object = manager.output(fixture.client(id), &outputs[0].output);
    let description = object.get_image_description(
        &fixture.client(id).qh(),
        casement_testclient::surface::subsurface::H1Data,
    );
    let info = information(fixture.client(id), &description);
    fixture.pump();
    assert!(fixture.client(id).protocol_error().is_none());
    assert_eq!(names(&mut fixture, id, &description), ["ready2"]);
    let events = seen(fixture.client(id), &info);
    assert_eq!(events.last().map(|(event, _)| event.as_str()), Some("done"));
    let (first, args) = &events[0];
    assert_eq!(first, "primaries");
    assert_eq!(args[..2], ["640000", "330000"]);
}

/// A description a client made does not allow `get_information`:
/// wp_image_description_v1.no_information (1).
#[test]
fn a_clients_own_description_has_no_information() {
    let (mut fixture, id) = boot();
    let manager = ColorManagement::bind(fixture.client(id), 2);
    let creator = manager.creator(fixture.client(id));
    let description = describe(
        fixture.client(id),
        creator,
        Primaries::Srgb,
        TransferFunction::Gamma22,
    );
    let _info = information(fixture.client(id), &description);
    fixture.pump();
    assert_eq!(
        fault(&mut fixture, id),
        ("wp_image_description_v1".to_owned(), 1)
    );
}

/// A surface takes a ready description at commit and gives it up again; a second object for
/// the surface is wp_color_manager_v1.surface_exists (1).
#[test]
fn a_surface_is_tagged_and_untagged() {
    let (mut fixture, id) = boot();
    let client = fixture.client(id);
    let surface = Compositor::bind(client, 5).surface(client);
    let manager = ColorManagement::bind(client, 2);
    let object = manager.surface(client, &surface);
    let creator = manager.creator(client);
    let description = describe(
        client,
        creator,
        Primaries::DisplayP3,
        TransferFunction::Gamma22,
    );
    fixture.pump();
    object.set_image_description(&description, RenderIntent::Perceptual);
    surface.commit();
    object.unset_image_description();
    surface.commit();
    fixture.pump();
    assert!(fixture.client(id).protocol_error().is_none());
    let client = fixture.client(id);
    let _again = manager.surface(client, &surface);
    fixture.pump();
    assert_eq!(
        fault(&mut fixture, id),
        ("wp_color_manager_v1".to_owned(), 1)
    );
}

/// Only the perceptual intent is served: wp_color_management_surface_v1.render_intent (0).
#[test]
fn another_intent_is_a_protocol_error() {
    let (mut fixture, id) = boot();
    let client = fixture.client(id);
    let surface = Compositor::bind(client, 5).surface(client);
    let manager = ColorManagement::bind(client, 2);
    let object = manager.surface(client, &surface);
    let creator = manager.creator(client);
    let description = describe(client, creator, Primaries::Srgb, TransferFunction::Gamma22);
    fixture.pump();
    object.set_image_description(&description, RenderIntent::Saturation);
    fixture.pump();
    assert_eq!(
        fault(&mut fixture, id),
        ("wp_color_management_surface_v1".to_owned(), 0)
    );
}
