//! `fractional_scale` (P0-H1).

use casement_testclient::misc::fractional::{FractionalScale, preferred};
use casement_testclient::surface::subsurface::{Compositor, Surface};
use casement_testclient::surface::toplevel::WmBase;

use casement_core::policy::ConnectionRole;
use casement_testclient::TestClient;
use casement_testkit::{ClientId, Env, Fixture, FixtureSpec};

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

const SCALE_1_5: &str = "HEADLESS-1:1920x1080@60000:180:0,0";

fn boot_at(outputs: &[&str]) -> (Fixture, ClientId) {
    let mut fixture =
        Fixture::start(FixtureSpec::desktop().with_outputs(outputs)).expect("fixture");
    let id = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    (fixture, id)
}

/// At scale 1.5 the surface receives preferred_scale(180) before its first xdg configure (shell-host FINDINGS 167).
#[test]
fn preferred_scale_comes_before_the_first_configure() {
    let (mut fixture, id) = boot_at(&[SCALE_1_5]);
    let base = WmBase::bind(fixture.client(id), 6);
    let top = base.toplevel(fixture.client(id));
    let client = fixture.client(id);
    let manager = FractionalScale::bind(client);
    let surface = Surface {
        surface: top.surface.clone(),
    };
    let object = manager.for_surface(client, &surface);
    top.commit_bare();
    fixture.pump();
    assert_eq!(preferred(fixture.client(id), &object), vec![180]);
    let lines = fixture.client(id).transcript().0.clone();
    let first = |object: &str, event: &str| {
        lines
            .iter()
            .position(|line| line.object.starts_with(object) && line.event == event)
    };
    let scale = first("wp_fractional_scale_v1#", "preferred_scale").expect("a preferred scale");
    let configure = first("xdg_toplevel#", "configure").expect("the first configure");
    assert!(
        scale < configure,
        "preferred_scale (line {scale}) comes before xdg_toplevel.configure (line {configure})"
    );
}

/// A surface learns its scale when its scale object is created, from the primary output, and
/// the transcript matches tests/golden/transcripts/fractional-scale-preferred.txt.
#[test]
fn a_surface_learns_the_primary_outputs_scale() {
    let (mut fixture, id) = boot_at(&[SCALE_1_5]);
    let client = fixture.client(id);
    let compositor = Compositor::bind(client, 5);
    let manager = FractionalScale::bind(client);
    let surface = compositor.surface(client);
    let object = manager.for_surface(client, &surface);
    fixture.pump();
    assert_eq!(preferred(fixture.client(id), &object), vec![180]);
    check(
        "fractional-scale-preferred.txt",
        fixture.client(id),
        &["wl_registry", "wl_display"],
    );
}

/// The primary output is the top-left one: a second output to its right does not change it.
#[test]
fn the_primary_output_decides() {
    let (mut fixture, id) = boot_at(&[
        "HEADLESS-2:3840x2160@60000:240:1920,0",
        "HEADLESS-1:1920x1080@60000:150:0,0",
    ]);
    let client = fixture.client(id);
    let compositor = Compositor::bind(client, 5);
    let manager = FractionalScale::bind(client);
    let surface = compositor.surface(client);
    let object = manager.for_surface(client, &surface);
    fixture.pump();
    assert_eq!(preferred(fixture.client(id), &object), vec![150]);
}

/// A second scale object on one surface is wp_fractional_scale_manager_v1.fractional_scale_exists
/// (0), posted on the surface.
#[test]
fn a_second_scale_object_is_a_protocol_error() {
    let (mut fixture, id) = boot_at(&[SCALE_1_5]);
    let client = fixture.client(id);
    let compositor = Compositor::bind(client, 5);
    let manager = FractionalScale::bind(client);
    let surface = compositor.surface(client);
    let _first = manager.for_surface(client, &surface);
    let _second = manager.for_surface(client, &surface);
    fixture.pump();
    assert_eq!(fault(&mut fixture, id), ("wl_surface".to_owned(), 0));
}
