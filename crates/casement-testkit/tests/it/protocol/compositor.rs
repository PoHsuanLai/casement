//! `compositor` (P0-H1).

use casement_core::geometry::PhysicalSize;
use casement_testclient::surface::subsurface::{Compositor, H1Data};
use casement_testclient::{ClientError, Pattern, shm_buffer};
use wayland_client::protocol::wl_compositor::WlCompositor;

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

const SIZE: PhysicalSize = PhysicalSize { w: 64, h: 32 };

/// Wl_compositor binds at versions 1 to 6; a surface with a buffer commits; the transcript matches tests/golden/transcripts/compositor-commit.txt.
#[test]
fn every_version_binds_and_a_surface_commits() {
    let (mut fixture, _) = boot();
    for version in 1..=6 {
        let id = fixture.connect(ConnectionRole::Public);
        fixture.pump();
        let client = fixture.client(id);
        let compositor = Compositor::bind(client, version);
        let surface = compositor.surface(client);
        let buffer = shm_buffer(client, SIZE, Pattern::Gradient);
        surface.attach(&buffer);
        surface.damage_all();
        surface.commit();
        fixture.pump();
        let client = fixture.client(id);
        assert!(client.protocol_error().is_none(), "version {version}");
        // An advertised version is the ceiling: asking for the next one is refused.
        assert!(matches!(
            client.bind::<WlCompositor, _>(7..=7, H1Data),
            Err(ClientError::MissingGlobal(_))
        ));
    }
}

/// The canonical flow's events (a buffer's pool, a surface, an attach and a commit) are the
/// formats `wl_shm` announces and nothing else.
#[test]
fn the_commit_flow_transcript_matches_its_golden() {
    let (mut fixture, id) = boot();
    let client = fixture.client(id);
    let buffer = shm_buffer(client, SIZE, Pattern::Gradient);
    let compositor = Compositor::bind(client, 5);
    let surface = compositor.surface(client);
    surface.attach(&buffer);
    surface.damage_all();
    surface.commit();
    fixture.pump();
    check(
        "compositor-commit.txt",
        fixture.client(id),
        &["wl_registry", "wl_display"],
    );
}

/// A commit on a surface with no role yet changes nothing in the model.
#[test]
fn a_commit_without_a_role_is_unassigned() {
    let (mut fixture, id) = boot();
    let before = fixture.model().clone();
    let client = fixture.client(id);
    let surface = Compositor::bind(client, 5).surface(client);
    let buffer = shm_buffer(client, SIZE, Pattern::Solid(0xff00_8000));
    surface.attach(&buffer);
    surface.commit();
    fixture.pump();
    assert!(fixture.client(id).protocol_error().is_none());
    assert_eq!(*fixture.model(), before);
}

/// Destroying a surface before its buffer, and the reverse, leave no texture behind.
#[test]
fn destroying_a_surface_before_its_buffer_is_clean() {
    let (mut fixture, first) = boot();
    let second = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let before = fixture.model().clone();
    {
        let client = fixture.client(first);
        let surface = Compositor::bind(client, 5).surface(client);
        let buffer = shm_buffer(client, SIZE, Pattern::Gradient);
        surface.attach(&buffer);
        surface.commit();
        surface.destroy();
        drop(buffer);
    }
    {
        let client = fixture.client(second);
        let surface = Compositor::bind(client, 5).surface(client);
        let buffer = shm_buffer(client, SIZE, Pattern::Gradient);
        surface.attach(&buffer);
        surface.commit();
        drop(buffer);
        surface.destroy();
    }
    fixture.pump();
    for id in [first, second] {
        assert!(fixture.client(id).protocol_error().is_none());
        assert!(fixture.client(id).is_connected());
    }
    assert_eq!(*fixture.model(), before);
}

/// A buffer scale below one is wl_surface.invalid_scale (0) and disconnects only that client.
#[test]
fn a_zero_buffer_scale_is_a_protocol_error() {
    let (mut fixture, bad) = boot();
    let good = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let client = fixture.client(bad);
    let surface = Compositor::bind(client, 5).surface(client);
    surface.surface.set_buffer_scale(0);
    fixture.pump();
    assert_eq!(fault(&mut fixture, bad), ("wl_surface".to_owned(), 0));
    assert!(!fixture.client(bad).is_connected());
    assert!(fixture.client(good).is_connected());
}

/// A non-zero attach offset on a version 5 surface is wl_surface.invalid_offset (3).
#[test]
fn an_attach_offset_at_version_five_is_a_protocol_error() {
    let (mut fixture, id) = boot();
    let client = fixture.client(id);
    let surface = Compositor::bind(client, 5).surface(client);
    surface.surface.attach(None, 1, 0);
    fixture.pump();
    assert_eq!(fault(&mut fixture, id), ("wl_surface".to_owned(), 3));
}

mod outputs_of_a_surface {
    //! `wl_surface.enter` and `leave`: a surface is on the outputs its drawn rectangle touches.

    use super::super::xdg_shell::{map_toplevel, settle};
    use casement_core::OutputName;
    use casement_core::geometry::PhysicalSize;
    use casement_core::policy::ConnectionRole;
    use casement_testclient::surface::layer::{
        Anchor, KeyboardInteractivity, Layer, LayerAsk, LayerSurface,
    };
    use casement_testclient::surface::popup::{Popup, PopupSpec};
    use casement_testclient::surface::subsurface::{BoundOutput, bind_outputs};
    use casement_testclient::surface::toplevel::WmBase;
    use casement_testclient::{Pattern, TestClient, shm_buffer};
    use casement_testkit::{ClientId, Fixture, FixtureSpec};
    use wayland_client::Proxy;
    use wayland_client::protocol::wl_surface::WlSurface;

    const TWO: [&str; 2] = [
        "HEADLESS-1:1920x1080@60000:120:0,0",
        "HEADLESS-2:1920x1080@60000:120:1920,0",
    ];

    /// Two outputs side by side, a client that has bound both `wl_output`s.
    fn boot() -> (Fixture, ClientId, Vec<BoundOutput>) {
        let mut fixture =
            Fixture::start(FixtureSpec::desktop().with_outputs(&TWO)).expect("fixture");
        let id = fixture.connect(ConnectionRole::Public);
        fixture.pump();
        let bound = bind_outputs(fixture.client(id), 4, true).expect("outputs bind");
        fixture.pump();
        (fixture, id, bound)
    }

    /// The names of the outputs `surface` has entered and not left, from the transcript.
    fn on(client: &TestClient, surface: &WlSurface, bound: &[BoundOutput]) -> Vec<String> {
        let object = format!("wl_surface#{}", surface.id().protocol_id());
        let mut held: Vec<String> = Vec::new();
        for line in client.transcript().0.iter().filter(|l| l.object == object) {
            let Some(arg) = line.args.first() else {
                continue;
            };
            match line.event.as_str() {
                "enter" => held.push(arg.clone()),
                "leave" => held.retain(|one| one != arg),
                _ => {}
            }
        }
        let mut names: Vec<String> = bound
            .iter()
            .filter(|one| held.contains(&format!("wl_output#{}", one.output.id().protocol_id())))
            .filter_map(|one| one.facts(client).name)
            .collect();
        names.sort();
        names
    }

    fn names(of: &[&str]) -> Vec<String> {
        of.iter().map(|one| (*one).to_owned()).collect()
    }

    /// A mapped toplevel is on the output it is drawn on, and only that one.
    #[test]
    fn a_mapped_toplevel_enters_its_output() {
        let (mut fixture, id, bound) = boot();
        let base = WmBase::bind(fixture.client(id), 7);
        let top = map_toplevel(&mut fixture, id, &base, (400, 300));
        settle(&mut fixture);
        assert_eq!(
            on(fixture.client(id), top.wl_surface(), &bound),
            names(&["HEADLESS-1"])
        );
    }

    /// The bound `wl_output` of the output called `name`.
    fn output_named<'a>(
        fixture: &mut Fixture,
        id: ClientId,
        bound: &'a [BoundOutput],
        name: &str,
    ) -> &'a BoundOutput {
        bound
            .iter()
            .find(|one| one.facts(fixture.client(id)).name.as_deref() == Some(name))
            .expect("the output is bound")
    }

    /// A window that goes full screen on the other output enters it and leaves the first, and
    /// back again.
    #[test]
    fn a_toplevel_that_changes_output_enters_and_leaves() {
        let (mut fixture, id, bound) = boot();
        let base = WmBase::bind(fixture.client(id), 7);
        let top = map_toplevel(&mut fixture, id, &base, (400, 300));
        settle(&mut fixture);
        let second = output_named(&mut fixture, id, &bound, "HEADLESS-2")
            .output
            .clone();
        top.set_fullscreen_on(&second);
        fixture.pump();
        settle(&mut fixture);
        assert_eq!(
            on(fixture.client(id), top.wl_surface(), &bound),
            names(&["HEADLESS-2"])
        );
        // Leaving full screen keeps the window on the output it was on; asking for the first
        // output again takes it back: it enters that and leaves the second.
        top.unset_fullscreen();
        fixture.pump();
        settle(&mut fixture);
        let first = output_named(&mut fixture, id, &bound, "HEADLESS-1")
            .output
            .clone();
        top.set_fullscreen_on(&first);
        fixture.pump();
        settle(&mut fixture);
        assert_eq!(
            on(fixture.client(id), top.wl_surface(), &bound),
            names(&["HEADLESS-1"])
        );
    }

    /// A window on an output that is unplugged is told it left it.
    #[test]
    fn unplugging_an_output_leaves_it() {
        let (mut fixture, id, bound) = boot();
        let base = WmBase::bind(fixture.client(id), 7);
        let top = map_toplevel(&mut fixture, id, &base, (400, 300));
        let second = output_named(&mut fixture, id, &bound, "HEADLESS-2")
            .output
            .clone();
        top.set_fullscreen_on(&second);
        fixture.pump();
        settle(&mut fixture);
        assert_eq!(
            on(fixture.client(id), top.wl_surface(), &bound),
            names(&["HEADLESS-2"])
        );
        fixture.unplug(&OutputName::parse("HEADLESS-2").expect("a name"));
        settle(&mut fixture);
        assert!(
            !on(fixture.client(id), top.wl_surface(), &bound).contains(&"HEADLESS-2".to_owned())
        );
    }

    /// A layer surface is on the output it was made for, not on the other.
    #[test]
    fn a_layer_surface_is_on_its_output() {
        let (mut fixture, id, bound) = boot();
        let second = bound
            .iter()
            .find(|one| one.facts(fixture.client(id)).name.as_deref() == Some("HEADLESS-2"))
            .expect("the second output");
        let ask = LayerAsk {
            level: Layer::Top,
            namespace: "bar",
            anchor: Anchor::Top | Anchor::Left | Anchor::Right,
            exclusive_zone: 30,
            keyboard: KeyboardInteractivity::None,
            size: (0, 30),
        };
        let layer = LayerSurface::create_on(fixture.client(id), &ask, Some(&second.output))
            .expect("a layer surface");
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
        settle(&mut fixture);
        assert_eq!(
            on(fixture.client(id), layer.surface(), &bound),
            names(&["HEADLESS-2"])
        );
    }

    /// A popup is on the outputs its parent is on, and follows it when the parent changes output.
    #[test]
    fn a_popup_is_on_its_parents_outputs() {
        let (mut fixture, id, bound) = boot();
        let base = WmBase::bind(fixture.client(id), 7);
        let parent = map_toplevel(&mut fixture, id, &base, (400, 300));
        let surface = fixture.client(id).create_surface().expect("a surface");
        let popup = Popup::of(
            fixture.client(id),
            &base,
            &parent,
            surface,
            PopupSpec::menu_at((10, 10)),
        );
        popup.surface.commit();
        fixture.pump();
        assert!(popup.ack_latest().is_some(), "the popup's configure");
        let buffer = shm_buffer(
            fixture.client(id),
            PhysicalSize { w: 100, h: 50 },
            Pattern::Solid(0xff20_2020),
        );
        popup.show(&buffer);
        fixture.pump();
        settle(&mut fixture);
        assert_eq!(
            on(fixture.client(id), &popup.surface, &bound),
            names(&["HEADLESS-1"])
        );
        let second = output_named(&mut fixture, id, &bound, "HEADLESS-2")
            .output
            .clone();
        parent.set_fullscreen_on(&second);
        fixture.pump();
        settle(&mut fixture);
        assert_eq!(
            on(fixture.client(id), parent.wl_surface(), &bound),
            names(&["HEADLESS-2"])
        );
        assert_eq!(
            on(fixture.client(id), &popup.surface, &bound),
            names(&["HEADLESS-2"])
        );
    }
}
