//! `alpha_modifier` (P0-H1).

use casement_testclient::misc::viewporter::AlphaModifier;
use casement_testclient::surface::subsurface::Compositor;

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

/// A surface at half alpha composites at half (pixman golden).
#[test]
#[ignore = "P0-R-scene: a pixman golden of a mapped window (needs P0-H2's toplevel helper and P0-B-loop's frames)"]
fn the_multiplier_reaches_the_frame() {
    todo!(
        "P0-H1: map a toplevel of one colour, set the multiplier to half, and compare its pixels over a known backdrop"
    )
}

/// A multiplier is accepted and committed with no protocol error and no event.
#[test]
#[ignore = "P0-B: needs Fixture::start (P0-B-headless), the loop (P0-B-loop) and the client wiring of Fixture::connect"]
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
#[ignore = "P0-B: needs Fixture::start (P0-B-headless), the loop (P0-B-loop) and the client wiring of Fixture::connect"]
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
