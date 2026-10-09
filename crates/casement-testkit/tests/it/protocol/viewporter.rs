//! `viewporter` (P0-H1).

use casement_core::geometry::PhysicalSize;
use casement_testclient::misc::viewporter::Viewporter;
use casement_testclient::surface::subsurface::Compositor;
use casement_testclient::{Pattern, shm_buffer};

use casement_core::policy::ConnectionRole;
use casement_testclient::TestClient;
use casement_testkit::{ClientId, Env, Fixture, FixtureSpec};

use crate::protocol::shoot;

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
fn a_viewport_crops_and_scales() {
    let (mut fixture, id, base) = shoot::desk();
    let (_top, _kept) = shoot::map_with(&mut fixture, id, &base, |client, top| {
        let buffer = shm_buffer(client, SIZE, Pattern::Gradient);
        let viewport = Viewporter::bind(client).viewport(client, &shoot::surface_of(top));
        viewport.source(16.0, 16.0, 32.0, 32.0);
        viewport.destination(128, 128);
        top.surface.attach(Some(buffer.buffer()), 0, 0);
        (buffer, viewport)
    });
    let window = shoot::windows(&fixture)[0];
    let shot = shoot::shot_of(&mut fixture, window);
    assert_eq!(shot.dimensions(), (128, 128), "the destination is the size");
    // The crop starts at buffer pixel (16, 16): red and green are a quarter of the way up their
    // ramps there (the gradient is 64 wide, so 16 / 63 of 255), and the far corner is at 47 / 63.
    let near = |pixel: [u8; 4], red: u8, green: u8| {
        pixel[0].abs_diff(red) <= 6 && pixel[1].abs_diff(green) <= 6
    };
    assert!(
        near(shot.get_pixel(1, 1).0, 65, 65),
        "{:?}",
        shot.get_pixel(1, 1)
    );
    assert!(
        near(shot.get_pixel(126, 126).0, 190, 190),
        "{:?}",
        shot.get_pixel(126, 126)
    );
    shoot::check_pixels("viewport-crop", &shot);
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

/// A second viewport on one surface is wp_viewporter.viewport_exists (0), posted on the manager.
#[test]
fn a_second_viewport_is_a_protocol_error() {
    let (mut fixture, id) = boot();
    let client = fixture.client(id);
    let surface = Compositor::bind(client, 5).surface(client);
    let viewporter = Viewporter::bind(client);
    let _first = viewporter.viewport(client, &surface);
    let _second = viewporter.viewport(client, &surface);
    fixture.pump();
    assert_eq!(fault(&mut fixture, id), ("wp_viewporter".to_owned(), 0));
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

/// A viewport made after the surface committed a buffer still gets `bad_size`: smithay inserts the
/// surface's viewport state at the first commit, and the check must not depend on who inserted it.
#[test]
fn a_viewport_made_after_the_first_commit_is_checked_for_bad_size() {
    let (mut fixture, id) = boot();
    let client = fixture.client(id);
    let surface = Compositor::bind(client, 5).surface(client);
    let buffer = shm_buffer(client, SIZE, Pattern::Gradient);
    surface.attach(&buffer);
    surface.commit();
    fixture.pump();
    let client = fixture.client(id);
    let viewport = Viewporter::bind(client).viewport(client, &surface);
    viewport.source(0.0, 0.0, 10.5, 10.5);
    surface.commit();
    fixture.pump();
    assert_eq!(fault(&mut fixture, id), ("wp_viewport".to_owned(), 1));
}

/// A commit that sets only the buffer scale changes the surface size, and a source that no longer
/// fits is wp_viewport.out_of_buffer (2): the check runs at every commit, not only with a buffer.
#[test]
fn a_scale_change_without_a_new_buffer_is_checked_for_out_of_buffer() {
    let (mut fixture, id) = boot();
    let client = fixture.client(id);
    let surface = Compositor::bind(client, 5).surface(client);
    let viewport = Viewporter::bind(client).viewport(client, &surface);
    let buffer = shm_buffer(client, SIZE, Pattern::Gradient);
    surface.attach(&buffer);
    viewport.source(0.0, 0.0, 64.0, 64.0);
    surface.commit();
    fixture.pump();
    assert!(
        fixture.client(id).protocol_error().is_none(),
        "it fits at scale 1"
    );
    surface.surface.set_buffer_scale(2);
    surface.commit();
    fixture.pump();
    assert_eq!(fault(&mut fixture, id), ("wp_viewport".to_owned(), 2));
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
