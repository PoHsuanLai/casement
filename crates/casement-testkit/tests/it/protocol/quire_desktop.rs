//! `quire_desktop` (P2-Q1): the desktop model of apps, windows and Spaces, read by the shell and
//! the agent tier, managed by the shell alone. The shell sees the whole desktop on bind (a
//! transcript golden), its requests reach the model and come back as events, an agent connection
//! reads but is refused every request that changes something, and what goes away is followed.

use crate::protocol::wlr_toplevel::flow;
use crate::protocol::xdg_shell::{boot, check_golden, golden, map_named, windows};
use casement_core::app::AppVisibility as Held;
use casement_core::policy::ConnectionRole;
use casement_core::window::{SpacePlace, WindowState};
use casement_testclient::ClientError;
use casement_testclient::quire::desktop::{
    AppLifeKind, AppRequest, AppVisibility, Assign, DesktopWatch, ManagerError, PlaceKind,
    PlaceTarget, PlacementMark, Presence, ProofKind, SpaceKind, SpaceRequest, Toggle,
    WindowRequest, WindowStateKind,
};
use casement_testclient::surface::toplevel::{Toplevel, WmBase};
use casement_testkit::{ClientId, Fixture, FixtureSpec};
use porter_core::{SandboxFacts, SpaceId};

const INTERFACE: &str = "quire_desktop_manager_v1";
const APP: &str = "org.example.Mail";

/// Whether the client `id` is offered `interface`.
fn sees(fixture: &mut Fixture, id: casement_testkit::ClientId, interface: &str) -> bool {
    fixture
        .client(id)
        .globals()
        .contents()
        .with_list(|list| list.iter().any(|global| global.interface == interface))
}

/// A started desktop where a public client has mapped `count` windows of one app.
fn mapped(count: usize) -> (Fixture, ClientId, Vec<Toplevel>) {
    let (mut fixture, public) = boot();
    let base = WmBase::bind(fixture.client(public), 7);
    let tops = (0..count)
        .map(|_| map_named(&mut fixture, public, &base, (400, 300), Some(APP)))
        .collect();
    (fixture, public, tops)
}

/// Pumps the compositor and the desktop client until both are quiet.
fn settle(fixture: &mut Fixture, watch: &mut DesktopWatch) {
    for _ in 0..3 {
        fixture.pump();
        watch.pump().expect("pump");
    }
}

/// A client of `role` with a bound desktop manager, all read.
fn watched(fixture: &mut Fixture, role: ConnectionRole) -> (ClientId, DesktopWatch) {
    let id = fixture.connect(role);
    fixture.pump();
    let mut watch = DesktopWatch::start(fixture.client(id).globals()).expect("the manager");
    settle(fixture, &mut watch);
    (id, watch)
}

/// Makes a request, then pumps both ends until the answer is read.
fn ask(
    fixture: &mut Fixture,
    watch: &mut DesktopWatch,
    request: impl FnOnce(&DesktopWatch) -> Result<(), ClientError>,
) {
    request(watch).expect("the request is sent");
    settle(fixture, watch);
}

/// The Spaces the manager has not ended.
fn live_spaces(watch: &DesktopWatch) -> usize {
    watch
        .spaces()
        .iter()
        .filter(|space| space.presence == Presence::Here)
        .count()
}

/// The shell and the agent tier are offered the manager; the public socket and a sandbox are not
/// (I7).
#[test]
fn only_the_shell_and_the_agent_tier_see_the_manager() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let public = fixture.connect(ConnectionRole::Public);
    let shell = fixture.connect(ConnectionRole::Shell);
    let agent = fixture.connect(ConnectionRole::Agent);
    let sandboxed = fixture.connect(ConnectionRole::Sandboxed(SandboxFacts::Flatpak {
        app: "org.example.App".to_owned(),
        instance: "1".to_owned(),
    }));
    fixture.pump();
    assert!(sees(&mut fixture, shell, INTERFACE), "the shell");
    assert!(sees(&mut fixture, agent, INTERFACE), "the agent tier");
    assert!(!sees(&mut fixture, public, INTERFACE), "the public socket");
    assert!(!sees(&mut fixture, sandboxed, INTERFACE), "a sandbox");
}

/// Binding sends an `app`, `window` and `space` event for every existing object, each followed
/// by its state burst, then `active` and `done`; the transcript is a golden.
#[test]
fn binding_sends_the_whole_desktop() {
    let (mut fixture, _public, _tops) = mapped(2);
    let (shell, watch) = watched(&mut fixture, ConnectionRole::Shell);
    let space = fixture
        .model()
        .spaces()
        .iter()
        .next()
        .expect("a Space")
        .id
        .as_str()
        .to_owned();

    let spaces = watch.spaces();
    assert_eq!(spaces.len(), 1);
    assert_eq!(spaces[0].id, space, "the Space's id is the minted id");
    assert_eq!(spaces[0].kind, Some(SpaceKind::Desktop));
    assert_eq!(spaces[0].position, Some(0));
    assert_eq!((spaces[0].outputs, spaces[0].dones), (1, 1));

    let apps = watch.apps();
    assert_eq!(apps.len(), 1);
    assert_eq!(apps[0].key, "claimed:org.example.Mail");
    assert_eq!(apps[0].proof, Some(ProofKind::Claimed));
    assert_eq!(apps[0].claimed_id, APP);
    assert_eq!(apps[0].life, Some(AppLifeKind::Running));
    assert_eq!(apps[0].visibility, Some(AppVisibility::Shown));
    assert_eq!(apps[0].mru, Some(0));
    assert_eq!(apps[0].dones, 1);

    let listed = watch.windows();
    assert_eq!(listed.len(), 2);
    for (window, id) in listed.iter().zip(windows(&fixture)) {
        assert_eq!(window.identifier, format!("w{}", id.0));
        assert_eq!(window.app.as_deref(), Some(apps[0].key.as_str()));
        assert_eq!(window.state, Some(WindowStateKind::Floating));
        assert_eq!(window.place, Some(PlaceKind::OnSpace));
        assert_eq!(window.space.as_deref(), Some(space.as_str()));
        assert_eq!(window.placement, Some(PlacementMark::Free));
        assert_eq!(window.outputs, 1);
        assert_eq!(window.dones, 1);
        assert!(window.role.is_some() && window.emphasis.is_some());
        let geometry = window.geometry.expect("an arranged window has geometry");
        assert_eq!((geometry.width, geometry.height), (400, 300));
        assert_eq!(geometry.scale120, 120);
    }
    let key = fixture.model().focus().key.expect("a key window");
    assert_eq!(watch.active().app.as_deref(), Some(apps[0].key.as_str()));
    assert_eq!(watch.active().key, Some(format!("w{}", key.0)));
    assert_eq!(watch.dones(), 1, "one burst closes the bind");

    let text = flow(
        &mut fixture,
        shell,
        &["quire_desktop_"],
        &[
            "id",
            "identifier",
            "role",
            "geometry",
            "parent",
            "emphasis",
            "sensitivity",
            "active",
        ],
    );
    check_golden(&golden("quire_desktop-bind.txt"), &text).expect("the bind flow");
}

/// Every management request on an agent connection is `not_permitted`, the model is unchanged
/// and the reads keep working; `get_window_for` is a read.
#[test]
fn an_agent_connection_is_refused_management() {
    type Request = fn(&DesktopWatch) -> Result<(), ClientError>;
    let requests: [(&str, Request); 19] = [
        ("create_space", |w| w.create_space()),
        ("app.activate", |w| w.app(0, AppRequest::Activate)),
        ("app.hide", |w| w.app(0, AppRequest::Hide)),
        ("app.unhide", |w| w.app(0, AppRequest::Unhide)),
        ("app.hide_others", |w| w.app(0, AppRequest::HideOthers)),
        ("app.quit", |w| w.app(0, AppRequest::Quit)),
        ("app.force_quit", |w| w.app(0, AppRequest::ForceQuit)),
        ("app.reopen", |w| w.app(0, AppRequest::Reopen)),
        ("window.activate", |w| w.window(0, WindowRequest::Activate)),
        ("window.close", |w| w.window(0, WindowRequest::Close)),
        ("window.minimise", |w| w.window(0, WindowRequest::Minimise)),
        ("window.unminimise", |w| {
            w.window(0, WindowRequest::Unminimise)
        }),
        ("window.place", |w| {
            w.window(0, WindowRequest::Place(PlaceTarget::Left))
        }),
        ("window.full_screen", |w| {
            w.window(0, WindowRequest::FullScreen(Toggle::On))
        }),
        ("window.to_space", |w| {
            w.window(0, WindowRequest::ToSpace(0))
        }),
        ("window.assign", |w| {
            w.window(0, WindowRequest::Assign(Assign::AllSpaces))
        }),
        ("space.activate", |w| w.space(0, SpaceRequest::Activate)),
        ("space.remove", |w| w.space(0, SpaceRequest::Remove)),
        ("space.move_to", |w| w.space(0, SpaceRequest::MoveTo(1))),
    ];
    let (mut fixture, _public, tops) = mapped(1);
    let (_reader, mut reader_watch) = watched(&mut fixture, ConnectionRole::Agent);
    assert_eq!(reader_watch.apps().len(), 1, "an agent reads the apps");
    assert_eq!(reader_watch.windows().len(), 1, "and the windows");
    assert_eq!(reader_watch.spaces().len(), 1, "and the Spaces");
    reader_watch.get_window_for(0).expect("a read");
    settle(&mut fixture, &mut reader_watch);
    assert_eq!(reader_watch.asked_windows().len(), 1);
    let before = (
        fixture.model().windows().clone(),
        fixture.model().apps().clone(),
        fixture.model().spaces().clone(),
    );
    for (name, request) in requests {
        let (agent, mut watch) = watched(&mut fixture, ConnectionRole::Agent);
        request(&watch).expect("the request is sent");
        settle(&mut fixture, &mut watch);
        let error = fixture
            .client(agent)
            .protocol_error()
            .unwrap_or_else(|| panic!("{name} is refused"));
        assert_eq!(error.code, ManagerError::NotPermitted as u32, "{name}");
    }
    assert_eq!(
        before,
        (
            fixture.model().windows().clone(),
            fixture.model().apps().clone(),
            fixture.model().spaces().clone(),
        ),
        "no refused request changed the model"
    );
    assert_eq!(
        tops[0].log.with(|seen| seen.closes),
        0,
        "nor closed a window"
    );
}

/// The shell's requests reach the model the way the window's own requests do, and every change
/// comes back as an event.
#[test]
fn the_shells_requests_reach_the_model() {
    let (mut fixture, _public, tops) = mapped(2);
    let (shell, mut watch) = watched(&mut fixture, ConnectionRole::Shell);
    let ids = windows(&fixture);
    let key = fixture
        .model()
        .apps()
        .keys()
        .next()
        .expect("an app")
        .clone();
    let output = fixture
        .model()
        .spaces()
        .iter()
        .next()
        .expect("a Space")
        .output
        .clone();
    let first = ids[0];
    let second = ids[1];

    // place, minimise, unminimise
    ask(&mut fixture, &mut watch, |w| {
        w.window(0, WindowRequest::Place(PlaceTarget::Left))
    });
    assert!(matches!(
        fixture.model().windows()[&first].state,
        WindowState::Tiled { .. }
    ));
    assert_eq!(watch.windows()[0].state, Some(WindowStateKind::Tiled));
    assert_eq!(watch.windows()[0].placement, Some(PlacementMark::Left));
    ask(&mut fixture, &mut watch, |w| {
        w.window(0, WindowRequest::Minimise)
    });
    assert!(matches!(
        fixture.model().windows()[&first].state,
        WindowState::Minimised { .. }
    ));
    assert_eq!(watch.windows()[0].state, Some(WindowStateKind::Minimised));
    ask(&mut fixture, &mut watch, |w| {
        w.window(0, WindowRequest::Unminimise)
    });
    assert_eq!(watch.windows()[0].state, Some(WindowStateKind::Tiled));

    // hide and unhide the app
    ask(&mut fixture, &mut watch, |w| w.app(0, AppRequest::Hide));
    assert!(matches!(
        fixture.model().apps()[&key].visibility,
        Held::Hidden { .. }
    ));
    assert_eq!(watch.apps()[0].visibility, Some(AppVisibility::Hidden));
    ask(&mut fixture, &mut watch, |w| w.app(0, AppRequest::Unhide));
    assert_eq!(watch.apps()[0].visibility, Some(AppVisibility::Shown));
    ask(&mut fixture, &mut watch, |w| {
        w.app(0, AppRequest::HideOthers)
    });
    assert!(matches!(
        fixture.model().apps()[&key].visibility,
        Held::Shown
    ));

    // full screen takes a Space of its own and gives it back
    ask(&mut fixture, &mut watch, |w| {
        w.window(1, WindowRequest::FullScreen(Toggle::On))
    });
    assert!(matches!(
        fixture.model().windows()[&second].state,
        WindowState::FullScreen { .. }
    ));
    assert_eq!(watch.spaces().len(), 2);
    assert_eq!(watch.spaces()[1].kind, Some(SpaceKind::FullScreen));
    assert_eq!(watch.windows()[1].state, Some(WindowStateKind::FullScreen));
    ask(&mut fixture, &mut watch, |w| {
        w.window(1, WindowRequest::FullScreen(Toggle::Off))
    });
    assert_eq!(watch.spaces()[1].presence, Presence::Gone);
    assert_eq!(live_spaces(&watch), 1);

    // create_space, to_space, space.activate and move_to
    ask(&mut fixture, &mut watch, |w| w.create_space());
    assert_eq!(live_spaces(&watch), 2);
    assert_eq!(fixture.model().spaces().row(&output).len(), 2);
    let made = watch.spaces()[2].id.clone();
    let made_id = SpaceId::parse(&made).expect("a minted id");
    ask(&mut fixture, &mut watch, |w| {
        w.window(1, WindowRequest::ToSpace(2))
    });
    assert_eq!(
        fixture.model().windows()[&second].place,
        SpacePlace::On(made_id.clone())
    );
    assert_eq!(watch.windows()[1].space.as_deref(), Some(made.as_str()));
    ask(&mut fixture, &mut watch, |w| {
        w.space(2, SpaceRequest::Activate)
    });
    assert_eq!(
        fixture
            .model()
            .spaces()
            .shown_on(&output)
            .map(|s| s.id.clone()),
        Some(made_id.clone())
    );
    ask(&mut fixture, &mut watch, |w| {
        w.space(0, SpaceRequest::Activate)
    });
    ask(&mut fixture, &mut watch, |w| {
        w.space(2, SpaceRequest::MoveTo(0))
    });
    assert_eq!(fixture.model().spaces().row(&output)[0].id, made_id);
    assert_eq!(watch.spaces()[2].position, Some(0));
    assert_eq!(watch.spaces()[0].position, Some(1));

    // assign
    ask(&mut fixture, &mut watch, |w| {
        w.window(1, WindowRequest::Assign(Assign::AllSpaces))
    });
    assert_eq!(
        fixture.model().windows()[&second].place,
        SpacePlace::AllSpaces
    );
    assert_eq!(watch.windows()[1].place, Some(PlaceKind::AllSpaces));
    assert_eq!(watch.windows()[1].space, None);
    ask(&mut fixture, &mut watch, |w| {
        w.window(1, WindowRequest::Assign(Assign::ThisSpace))
    });
    assert!(matches!(
        fixture.model().windows()[&second].place,
        SpacePlace::On(_)
    ));

    // activation with the serial 0 the desktop client sends is the shell's hand
    ask(&mut fixture, &mut watch, |w| {
        w.window(0, WindowRequest::Activate)
    });
    ask(&mut fixture, &mut watch, |w| w.app(0, AppRequest::Activate));
    assert!(fixture.client(shell).protocol_error().is_none());
    assert_eq!(fixture.model().focus().active.as_ref(), Some(&key));

    // a second view of a window follows it too
    watch.get_window_for(0).expect("the window list is offered");
    settle(&mut fixture, &mut watch);
    assert_eq!(
        watch.asked_windows()[0].identifier,
        watch.windows()[0].identifier
    );
    ask(&mut fixture, &mut watch, |w| {
        w.window(0, WindowRequest::Place(PlaceTarget::Right))
    });
    assert_eq!(
        watch.asked_windows()[0].placement,
        Some(PlacementMark::Right)
    );

    // close, quit, and the two that wait for their machines
    ask(&mut fixture, &mut watch, |w| {
        w.window(0, WindowRequest::Close)
    });
    assert_eq!(tops[0].log.with(|seen| seen.closes), 1);
    ask(&mut fixture, &mut watch, |w| w.app(0, AppRequest::Quit));
    assert_eq!(tops[0].log.with(|seen| seen.closes), 2, "asked again");
    assert_eq!(tops[1].log.with(|seen| seen.closes), 1);
    ask(&mut fixture, &mut watch, |w| {
        w.app(0, AppRequest::ForceQuit)
    });
    ask(&mut fixture, &mut watch, |w| w.app(0, AppRequest::Reopen));
    assert!(fixture.client(shell).protocol_error().is_none());
}

/// A window that closes is told `closed`; a Space that is removed is told `removed`; an app is
/// closed with its last client; a manager that sent `stop` gets `finished` and no new object.
#[test]
fn the_manager_follows_what_goes_away() {
    let (mut fixture, public, tops) = mapped(2);
    let (_shell, mut watch) = watched(&mut fixture, ConnectionRole::Shell);

    tops[0].destroy();
    settle(&mut fixture, &mut watch);
    assert_eq!(watch.windows()[0].presence, Presence::Gone);
    assert_eq!(watch.windows()[1].presence, Presence::Here);
    assert_eq!(
        watch.apps()[0].presence,
        Presence::Here,
        "the app still runs"
    );

    ask(&mut fixture, &mut watch, |w| w.create_space());
    assert_eq!(live_spaces(&watch), 2);
    ask(&mut fixture, &mut watch, |w| {
        w.space(1, SpaceRequest::Remove)
    });
    assert_eq!(watch.spaces()[1].presence, Presence::Gone);
    assert_eq!(live_spaces(&watch), 1);

    ask(&mut fixture, &mut watch, |w| w.stop());
    assert!(watch.finished(), "stop is answered with finished");
    let announced = watch.spaces().len();
    ask(&mut fixture, &mut watch, |w| w.create_space());
    assert_eq!(
        watch.spaces().len(),
        announced,
        "a stopped manager is told of no new Space"
    );

    fixture.disconnect(public);
    settle(&mut fixture, &mut watch);
    assert_eq!(watch.windows()[1].presence, Presence::Gone);
    assert_eq!(
        watch.apps()[0].presence,
        Presence::Gone,
        "a stopped manager still hears that what it has ended"
    );
}
