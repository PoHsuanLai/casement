//! `alpha_modifier` (P0-H1).

use casement_core::geometry::PhysicalSize;
use casement_testclient::misc::viewporter::AlphaModifier;
use casement_testclient::surface::subsurface::Compositor;
use casement_testclient::surface::toplevel::Toplevel;
use casement_testclient::{Pattern, shm_buffer};

use casement_core::policy::ConnectionRole;
use casement_testclient::TestClient;
use casement_testkit::{ClientId, Env, Fixture, FixtureSpec};

use crate::shoot;

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

/// The backdrop's and the front window's sizes.
const SIZE: (PhysicalSize, PhysicalSize) = (
    PhysicalSize { w: 300, h: 200 },
    PhysicalSize { w: 100, h: 100 },
);

/// A surface at half alpha composites at half (pixman golden).
#[test]
fn the_multiplier_reaches_the_frame() {
    let (mut fixture, id, base) = shoot::desk();
    let backdrop = |client: &mut TestClient, top: &Toplevel| {
        let buffer = shm_buffer(client, SIZE.0, Pattern::Solid(0xffff_0000));
        top.surface.attach(Some(buffer.buffer()), 0, 0);
        buffer
    };
    let (_below, _kept_below) = shoot::map_with(&mut fixture, id, &base, backdrop);
    let (_front, _kept_front) = shoot::map_with(&mut fixture, id, &base, |client, top| {
        let alpha = AlphaModifier::bind(client).for_surface(client, &shoot::surface_of(top));
        alpha.set_multiplier(0x8000_0000);
        let buffer = shm_buffer(client, SIZE.1, Pattern::Solid(0xff00_00ff));
        top.surface.attach(Some(buffer.buffer()), 0, 0);
        (alpha, buffer)
    });
    // The backdrop is the window whose shot is its own size (ids do not say which came first).
    let shot = shoot::windows(&fixture)
        .into_iter()
        .map(|window| shoot::shot_of(&mut fixture, window))
        .find(|shot| shot.dimensions() == (SIZE.0.w, SIZE.0.h))
        .unwrap_or_else(|| panic!("no shot of the backdrop"));
    // The front window is cascaded onto the backdrop; opaque blue at half is half blue over
    // half red, and the backdrop beside it is still red.
    let blended = shot
        .pixels()
        .filter(|pixel| {
            pixel.0[0].abs_diff(127) <= 2 && pixel.0[1] == 0 && pixel.0[2].abs_diff(127) <= 2
        })
        .count();
    let front_area = (SIZE.1.w * SIZE.1.h) as usize;
    assert_eq!(blended, front_area, "the front window blends at half");
    shoot::check_pixels("alpha-multiplier", &shot);
}

/// A multiplier is accepted and committed with no protocol error and no event.
#[test]
fn a_multiplier_is_accepted() {
    let (mut fixture, id) = boot();
    let client = fixture.client(id);
    let surface = Compositor::bind(client, 5).surface(client);
    let alpha = AlphaModifier::bind(client).for_surface(client, &surface);
    alpha.set_multiplier(0x8000_0000);
    surface.commit();
    alpha.set_multiplier(u32::MAX);
    surface.commit();
    alpha.destroy();
    fixture.pump();
    assert!(fixture.client(id).protocol_error().is_none());
    check(
        "alpha-modifier-flow.txt",
        fixture.client(id),
        &["wl_registry", "wl_display"],
    );
}

/// A second alpha object on one surface is wp_alpha_modifier_v1.already_constructed (0).
#[test]
fn a_second_alpha_object_is_a_protocol_error() {
    let (mut fixture, id) = boot();
    let client = fixture.client(id);
    let surface = Compositor::bind(client, 5).surface(client);
    let manager = AlphaModifier::bind(client);
    let _first = manager.for_surface(client, &surface);
    let _second = manager.for_surface(client, &surface);
    fixture.pump();
    assert_eq!(
        fault(&mut fixture, id),
        ("wp_alpha_modifier_v1".to_owned(), 0)
    );
}
