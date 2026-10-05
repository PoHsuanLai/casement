//! `viewporter` (P0-H1).

use casement_core::geometry::PhysicalSize;
use casement_testclient::misc::viewporter::Viewporter;
use casement_testclient::surface::subsurface::Compositor;
use casement_testclient::{Pattern, shm_buffer};

use casement_core::policy::ConnectionRole;
use casement_testclient::TestClient;
use casement_testkit::{ClientId, Env, Fixture, FixtureSpec};

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

/// Compares what `client` received, without the events of the objects of `skip` and without
/// instants, with `tests/golden/transcripts/<name>`; `CASEMENT_BLESS=1` replaces a differing
/// golden (CONVENTIONS addendum 12).
fn check(name: &str, client: &TestClient, skip: &[&str]) {
    let actual: String = client
        .transcript()
        .0
        .iter()
        .filter(|line| {
            !skip
                .iter()
                .any(|iface| line.object.starts_with(&format!("{iface}#")))
        })
        .map(|line| format!("{}.{}({})\n", line.object, line.event, line.args.join(", ")))
        .collect();
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden/transcripts")
        .join(name);
    let golden = std::fs::read_to_string(&path).unwrap_or_default();
    if golden == actual {
        return;
    }
    if Env::of_process().bless == casement_render::golden::Bless::Yes {
        std::fs::write(&path, &actual).expect("write the blessed golden");
        return;
    }
    panic!(
        "{} differs\n--- golden\n{golden}--- actual\n{actual}",
        path.display()
    );
}

const SIZE: PhysicalSize = PhysicalSize { w: 64, h: 64 };

/// A viewport's source and destination reach the frame (pixman golden).
#[test]
#[ignore = "body is todo!(): needs a pixman golden of a mapped toplevel (P0-R-scene shoot(); P0-H2's toplevel helper is on master)"]
fn a_viewport_crops_and_scales() {
    todo!(
        "P0-H1: map a toplevel with a 64x64 gradient, source (16,16,32,32), destination 128x128, and compare the frame with tests/golden/scenes/viewport-crop.png"
    )
}

/// Valid source and destination are accepted, committed and removed again without a protocol
/// error; the viewport object leaves no events.
#[test]
fn a_viewport_is_accepted_and_removed() {
    let (mut fixture, id) = boot();
    let client = fixture.client(id);
    let surface = Compositor::bind(client, 5).surface(client);
    let viewport = Viewporter::bind(client).viewport(client, &surface);
    let buffer = shm_buffer(client, SIZE, Pattern::CoordinateEncoding);
    surface.attach(&buffer);
    viewport.source(16.0, 16.0, 32.0, 32.0);
    viewport.destination(128, 128);
    surface.commit();
    // -1 -1 unsets the destination, -1 -1 -1 -1 the source.
    viewport.destination(-1, -1);
    viewport.source(-1.0, -1.0, -1.0, -1.0);
    surface.commit();
    viewport.destroy();
    surface.commit();
    fixture.pump();
    assert!(fixture.client(id).protocol_error().is_none());
    check(
        "viewporter-flow.txt",
        fixture.client(id),
        &["wl_registry", "wl_display", "wl_shm"],
    );
}

/// A second viewport on one surface is wp_viewporter.viewport_exists (0), posted on the surface.
#[test]
fn a_second_viewport_is_a_protocol_error() {
    let (mut fixture, id) = boot();
    let client = fixture.client(id);
    let surface = Compositor::bind(client, 5).surface(client);
    let viewporter = Viewporter::bind(client);
    let _first = viewporter.viewport(client, &surface);
    let _second = viewporter.viewport(client, &surface);
    fixture.pump();
    assert_eq!(fault(&mut fixture, id), ("wl_surface".to_owned(), 0));
}

/// A destination of zero, or a negative source origin, is wp_viewport.bad_value (0).
#[test]
fn bad_values_are_protocol_errors() {
    for (destination, source) in [(Some((0, 10)), None), (None, Some((-1.0, 0.0, 10.0, 10.0)))] {
        let (mut fixture, id) = boot();
        let client = fixture.client(id);
        let surface = Compositor::bind(client, 5).surface(client);
        let viewport = Viewporter::bind(client).viewport(client, &surface);
        if let Some((w, h)) = destination {
            viewport.destination(w, h);
        }
        if let Some((x, y, w, h)) = source {
            viewport.source(x, y, w, h);
        }
        fixture.pump();
        assert_eq!(fault(&mut fixture, id), ("wp_viewport".to_owned(), 0));
    }
}

/// A source outside the buffer is wp_viewport.out_of_buffer (2), raised at commit.
#[test]
fn a_source_outside_the_buffer_is_a_protocol_error() {
    let (mut fixture, id) = boot();
    let client = fixture.client(id);
    let surface = Compositor::bind(client, 5).surface(client);
    let viewport = Viewporter::bind(client).viewport(client, &surface);
    let buffer = shm_buffer(client, SIZE, Pattern::Gradient);
    surface.attach(&buffer);
    viewport.source(0.0, 0.0, 100.0, 100.0);
    surface.commit();
    fixture.pump();
    assert_eq!(fault(&mut fixture, id), ("wp_viewport".to_owned(), 2));
}

/// A fractional source without a destination is wp_viewport.bad_size (1).
#[test]
fn a_fractional_size_without_a_destination_is_a_protocol_error() {
    let (mut fixture, id) = boot();
    let client = fixture.client(id);
    let surface = Compositor::bind(client, 5).surface(client);
    let viewport = Viewporter::bind(client).viewport(client, &surface);
    let buffer = shm_buffer(client, SIZE, Pattern::Gradient);
    surface.attach(&buffer);
    viewport.source(0.0, 0.0, 10.5, 10.5);
    surface.commit();
    fixture.pump();
    assert_eq!(fault(&mut fixture, id), ("wp_viewport".to_owned(), 1));
}

/// Requests on a viewport whose surface is gone are wp_viewport.no_surface (3).
#[test]
fn a_viewport_outliving_its_surface_is_a_protocol_error() {
    let (mut fixture, id) = boot();
    let client = fixture.client(id);
    let surface = Compositor::bind(client, 5).surface(client);
    let viewport = Viewporter::bind(client).viewport(client, &surface);
    surface.destroy();
    viewport.destination(10, 10);
    fixture.pump();
    assert_eq!(fault(&mut fixture, id), ("wp_viewport".to_owned(), 3));
}
