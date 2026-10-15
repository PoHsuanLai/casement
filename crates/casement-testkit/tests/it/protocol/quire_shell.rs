//! `quire_shell` (P2-Q2): the shell's surface roles, minimise targets and the events only the
//! compositor sees. Shell connections only.

use crate::protocol::xdg_shell::{map_toplevel, pointer, rect_of, settle, windows};
use casement_core::WindowId;
use casement_core::geometry::PhysicalSize;
use casement_core::input::{DeviceClass, Edge, KeyCode};
use casement_core::policy::ConnectionRole;
use casement_core::space::SpaceContent;
use casement_core::window::{Light, MenuEntry, WindowState, menu_point};
use casement_ipc::dbus::ConnectRefusal;
use casement_layout::{GroupHover, Scheme, frame_style};
use casement_protocols::vocab::ShellRole as ModelRole;
use casement_testclient::misc::decoration::DecorationManager;
use casement_testclient::quire::shell::{
    ShellError, ShellHandle, ShellRole, SplitChoiceSeen, SplitSide, WindowEntry,
};
use casement_testclient::surface::layer::{
    Anchor, KeyboardInteractivity, Layer, LayerAsk, LayerSurface,
};
use casement_testclient::surface::toplevel::{Toplevel, WmBase};
use casement_testclient::{Pattern, shm_buffer};
use casement_testkit::fakes::FakeCgroups;
use casement_testkit::{ClientId, Fixture, FixtureSpec, TestOrigin};
use casement_wayland::foreign_toplevel_list::identifier_of;
use ds_core::vocab::Activity;
use porter_core::SandboxFacts;
use std::time::Duration;

const INTERFACE: &str = "quire_shell_v1";

/// Whether the client `id` is offered `interface`.
fn sees(fixture: &mut Fixture, id: ClientId, interface: &str) -> bool {
    fixture
        .client(id)
        .globals()
        .contents()
        .with_list(|list| list.iter().any(|global| global.interface == interface))
}

/// Only the shell's connection is offered `quire_shell_v1` (I7): not the agent tier, not the
/// public socket, not a sandbox.
#[test]
fn only_the_shell_sees_quire_shell() {
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
    assert!(!sees(&mut fixture, agent, INTERFACE), "the agent tier");
    assert!(!sees(&mut fixture, public, INTERFACE), "the public socket");
    assert!(!sees(&mut fixture, sandboxed, INTERFACE), "a sandbox");
}

/// A dock-like layer surface along the bottom edge.
fn dock_ask() -> LayerAsk {
    LayerAsk {
        level: Layer::Top,
        namespace: "dock",
        anchor: Anchor::Bottom,
        exclusive_zone: 0,
        keyboard: KeyboardInteractivity::None,
        size: (400, 80),
    }
}

/// Maps a layer surface of `id`: create, acknowledge the configure, attach a buffer.
fn map_layer(fixture: &mut Fixture, id: ClientId, ask: &LayerAsk) -> LayerSurface {
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

/// A shell connection with `quire_shell_v1` bound.
pub(super) fn shell_client(fixture: &mut Fixture) -> (ClientId, ShellHandle) {
    let id = fixture.connect(ConnectionRole::Shell);
    let shell = ShellHandle::bind(fixture.client(id), 1);
    fixture.pump();
    (id, shell)
}

/// A program is given the connection its unit may have (`Connect` over the fake process table):
/// the shell's unit gets `quire_shell_v1`, the agent tier's does not, and anything else is
/// refused with the reason the D-Bus caller is told.
#[test]
fn the_unit_check_decides_which_connection_a_program_is_handed() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let cgroups = FakeCgroups::new();
    cgroups.run_in_service(100, "sill.service");
    cgroups.run_in_service(101, "cuad.service");
    cgroups.run_in_service(102, "firefox.service");
    let shell = fixture
        .connect_through_handout(&cgroups, 100, ConnectionRole::Shell)
        .expect("sill gets the shell");
    let agent = fixture
        .connect_through_handout(&cgroups, 101, ConnectionRole::Agent)
        .expect("cuad gets the agent tier");
    assert!(sees(&mut fixture, shell, INTERFACE), "the shell");
    assert!(!sees(&mut fixture, agent, INTERFACE), "the agent tier");
    // (case, the process, the role asked, what comes back)
    let refused = [
        (
            "sill asks for the agent tier",
            100,
            ConnectionRole::Agent,
            ConnectRefusal::RoleNotAllowed,
        ),
        (
            "a browser asks for the shell",
            102,
            ConnectionRole::Shell,
            ConnectRefusal::UnknownUnit,
        ),
        (
            "a process nobody knows asks for the shell",
            999,
            ConnectionRole::Shell,
            ConnectRefusal::UnknownUnit,
        ),
    ];
    for (case, pid, role, why) in refused {
        assert_eq!(
            fixture.connect_through_handout(&cgroups, pid, role).err(),
            Some(why),
            "{case}"
        );
    }
}

/// A role goes to a layer surface of the client and to one surface at a time.
#[test]
fn a_role_is_given_to_one_layer_surface() {
    // The model holds the role, and it is free again once the surface unmaps.
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let (id, shell) = shell_client(&mut fixture);
    let first = map_layer(&mut fixture, id, &dock_ask());
    shell.set_role(first.surface(), ShellRole::Dock);
    fixture.pump();
    assert!(
        fixture
            .model()
            .layers()
            .role_holder(ModelRole::Dock)
            .is_some()
    );
    first.destroy(fixture.client(id));
    fixture.pump();
    assert!(
        fixture
            .model()
            .layers()
            .role_holder(ModelRole::Dock)
            .is_none()
    );
    let second = map_layer(&mut fixture, id, &dock_ask());
    shell.set_role(second.surface(), ShellRole::Dock);
    fixture.pump();
    assert!(fixture.client(id).protocol_error().is_none());
    assert!(
        fixture
            .model()
            .layers()
            .role_holder(ModelRole::Dock)
            .is_some()
    );

    // A second surface asking for a role that is taken is role_taken.
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let (id, shell) = shell_client(&mut fixture);
    let first = map_layer(&mut fixture, id, &dock_ask());
    let second = map_layer(&mut fixture, id, &dock_ask());
    shell.set_role(first.surface(), ShellRole::Dock);
    shell.set_role(second.surface(), ShellRole::Dock);
    fixture.pump();
    let error = fixture.client(id).protocol_error().expect("role_taken");
    assert_eq!(error.interface, "quire_shell_v1");
    assert_eq!(error.code, ShellError::RoleTaken as u32);

    // A surface that is no layer surface is bad_surface.
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let (id, shell) = shell_client(&mut fixture);
    let plain = fixture.client(id).create_surface().expect("a surface");
    shell.set_role(&plain, ShellRole::Bar);
    fixture.pump();
    let error = fixture.client(id).protocol_error().expect("bad_surface");
    assert_eq!(error.code, ShellError::BadSurface as u32);
}

/// A minimise target set by the dock decides where a window minimises to.
#[test]
fn the_dock_decides_where_a_window_minimises_to() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let app = fixture.connect(ConnectionRole::Public);
    let base = WmBase::bind(fixture.client(app), 7);
    let _top = map_toplevel(&mut fixture, app, &base, (400, 300));
    let window = windows(&fixture)[0];
    let (id, shell) = shell_client(&mut fixture);
    let dock = map_layer(&mut fixture, id, &dock_ask());
    shell.set_role(dock.surface(), ShellRole::Dock);
    shell.set_minimise_target(dock.surface(), &identifier_of(window), (10, 5), (64, 64));
    fixture.pump();
    let target = fixture
        .model()
        .minimise_target(window)
        .expect("a minimise target")
        .rect;
    assert_eq!((target.size.w, target.size.h), (64, 64));
    assert!(target.origin.y > 5, "global pixels, not local to the dock");
    shell.clear_minimise_target(&identifier_of(window));
    fixture.pump();
    assert!(fixture.model().minimise_target(window).is_none());

    // A window nobody has is unknown_toplevel.
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let (id, shell) = shell_client(&mut fixture);
    let dock = map_layer(&mut fixture, id, &dock_ask());
    shell.set_minimise_target(dock.surface(), "w999", (0, 0), (64, 64));
    fixture.pump();
    let error = fixture
        .client(id)
        .protocol_error()
        .expect("unknown_toplevel");
    assert_eq!(error.code, ShellError::UnknownToplevel as u32);
}

/// The evdev code of the left logo key, the Command key.
const KEY_LEFTMETA: KeyCode = KeyCode(125);
/// The evdev code of the A key.
const KEY_A: KeyCode = KeyCode(30);

/// The real modifier keys reach the shell's role surfaces, including when none of them has
/// keyboard focus (hot corners).
#[test]
fn modifiers_reach_the_shell_without_focus() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let app = fixture.connect(ConnectionRole::Public);
    let base = WmBase::bind(fixture.client(app), 7);
    let _top = map_toplevel(&mut fixture, app, &base, (400, 300));
    let (id, shell) = shell_client(&mut fixture);
    let bar = map_layer(&mut fixture, id, &dock_ask());
    shell.set_role(bar.surface(), ShellRole::Bar);
    fixture.pump();
    let device = fixture.input().plug(
        DeviceClass::Keyboard,
        TestOrigin::Physical(DeviceClass::Keyboard),
    );
    fixture.pump();
    let command = 8;
    fixture.input().key(device, KEY_LEFTMETA, Edge::Press);
    fixture.pump();
    assert_eq!(shell.modifiers_of(bar.surface()), [command]);
    // A key that changes no modifier sends nothing.
    fixture.input().key(device, KEY_A, Edge::Press);
    fixture.input().key(device, KEY_A, Edge::Release);
    fixture.pump();
    assert_eq!(shell.modifiers_of(bar.surface()), [command]);
    fixture.input().key(device, KEY_LEFTMETA, Edge::Release);
    fixture.pump();
    assert_eq!(shell.modifiers_of(bar.surface()), [command, 0]);
}

/// Maps a window whose client was answered server-side (the Mac frame is casement's): the
/// toplevel is decorated before its first commit, then it maps at `size`.
fn map_framed(fixture: &mut Fixture, id: ClientId, base: &WmBase, size: (u32, u32)) {
    let manager = DecorationManager::bind(fixture.client(id)).expect("xdg_decoration");
    let top = base.toplevel(fixture.client(id));
    let _decoration = manager.decorate(fixture.client(id), &top);
    top.commit_bare();
    fixture.pump();
    assert!(top.ack_latest().is_some(), "the initial configure arrives");
    let buffer = shm_buffer(
        fixture.client(id),
        PhysicalSize {
            w: size.0,
            h: size.1,
        },
        Pattern::Gradient,
    );
    top.show(&buffer);
    fixture.pump();
}

/// The pointer resting on the green light for half a second asks the shell for the window's
/// menu, at the light's left edge a little under it, with the entries the window's state
/// offers; a pick of Left tiles the window to the left half.
#[test]
fn resting_on_the_green_light_shows_the_menu_and_a_pick_tiles_the_window() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let app = fixture.connect(ConnectionRole::Public);
    let base = WmBase::bind(fixture.client(app), 7);
    map_framed(&mut fixture, app, &base, (400, 300));
    let window = windows(&fixture)[0];
    let (_, shell) = shell_client(&mut fixture);
    settle(&mut fixture);

    // The light is where the frame puts it round the content (the layout's rectangle).
    let content = fixture.framed_content_of(window);
    let style = frame_style(Scheme::Light, Activity::Active, GroupHover::Away);
    let light = style.around(content).light(Light::Zoom);
    let device = pointer(&mut fixture);
    fixture.input().pointer_to(device, light.centre());
    fixture.pump();
    assert!(
        shell.window_menus().is_empty(),
        "not before the rest is long enough"
    );
    fixture.advance(Duration::from_millis(300));
    assert!(shell.window_menus().is_empty(), "300 ms is not a rest");
    fixture.advance(Duration::from_millis(400));

    let menus = shell.window_menus();
    assert_eq!(menus.len(), 1, "one menu: {menus:?}");
    let at = menu_point(light);
    assert_eq!(menus[0].toplevel, identifier_of(window));
    assert_eq!(menus[0].at, (at.x, at.y));
    let on = |entry: MenuEntry| menus[0].enabled & (1 << entry.code()) != 0;
    assert!(
        on(MenuEntry::EnterFullScreen),
        "a window can enter full screen"
    );
    assert!(!on(MenuEntry::ExitFullScreen), "it is not in full screen");
    assert!(on(MenuEntry::Left), "Left is offered");
    assert!(!on(MenuEntry::ReturnToPreviousSize), "nothing to return to");
    assert!(
        on(MenuEntry::TileLeftOfScreen) && on(MenuEntry::TileRightOfScreen),
        "Full Screen Tile is offered now that Split View is built"
    );

    shell.window_menu_pick(&identifier_of(window), WindowEntry::Left);
    fixture.pump();
    assert!(matches!(
        fixture.model().windows()[&window].state,
        WindowState::Tiled { .. }
    ));
    assert!(fixture.client(app).protocol_error().is_none());
}

/// cas-wave "Real-run checks: Window menu" (a pick moves the window): each Move & Resize entry,
/// Fill and Center put the window where the entry says, on the 1920x1080 desktop (tile margins
/// leave a few pixels, so the bounds are loose), and Return to Previous Size undoes a tile.
#[test]
#[ignore = "wave-6 follow-up: tiled windows get no layout target until cas22-tile lands"]
fn a_pick_moves_and_resizes_the_window_as_its_entry_says() {
    // name, entry, whether the placed rectangle `(x, y, w, h)` is where the entry says
    type Check = fn((i32, i32, i32, i32)) -> bool;
    let cases: [(&str, WindowEntry, Check); 8] = [
        ("left half", WindowEntry::Left, |(x, _, w, h)| {
            x <= 20 && (900..=960).contains(&w) && h >= 1000
        }),
        ("right half", WindowEntry::Right, |(x, _, w, h)| {
            x >= 920 && x + w >= 1900 && (900..=960).contains(&w) && h >= 1000
        }),
        ("top half", WindowEntry::Top, |(_, y, w, h)| {
            y <= 20 && (480..=560).contains(&h) && w >= 1880
        }),
        ("bottom half", WindowEntry::Bottom, |(_, y, w, h)| {
            y >= 520 && y + h >= 1060 && (480..=560).contains(&h) && w >= 1880
        }),
        ("top left quarter", WindowEntry::TopLeft, |(x, y, w, h)| {
            x <= 20 && y <= 20 && (900..=960).contains(&w) && (480..=560).contains(&h)
        }),
        (
            "bottom right quarter",
            WindowEntry::BottomRight,
            |(x, y, w, h)| {
                x >= 920 && y >= 520 && x + w >= 1900 && y + h >= 1060 && w <= 960 && h <= 560
            },
        ),
        ("fill", WindowEntry::Fill, |(_, _, w, h)| {
            w >= 1880 && h >= 1000
        }),
        ("center", WindowEntry::Centre, |(x, y, w, h)| {
            (w, h) == (400, 300) && (x + w / 2 - 960).abs() <= 2 && (y + h / 2 - 540).abs() <= 60
        }),
    ];
    for (name, entry, placed) in cases {
        let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
        let app = fixture.connect(ConnectionRole::Public);
        let base = WmBase::bind(fixture.client(app), 7);
        let _top = map_toplevel(&mut fixture, app, &base, (400, 300));
        let window = windows(&fixture)[0];
        let (_, shell) = shell_client(&mut fixture);
        settle(&mut fixture);
        let before = rect_of(&fixture, window);
        shell.window_menu_pick(&identifier_of(window), entry);
        fixture.pump();
        let rect = rect_of(&fixture, window);
        let at = (
            rect.origin.x,
            rect.origin.y,
            i32::try_from(rect.size.w).expect("a small width"),
            i32::try_from(rect.size.h).expect("a small height"),
        );
        assert!(placed(at), "{name}: placed at {at:?}");

        // A tile can be undone from the same menu.
        if entry == WindowEntry::Left {
            shell.window_menu_pick(&identifier_of(window), WindowEntry::ReturnToPreviousSize);
            fixture.pump();
            assert_eq!(rect_of(&fixture, window), before, "return to previous size");
        }
    }
}

/// A pick naming no window, or an entry the XML does not name, changes nothing and is no
/// protocol error.
#[test]
fn a_pick_for_nothing_is_ignored() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let app = fixture.connect(ConnectionRole::Public);
    let base = WmBase::bind(fixture.client(app), 7);
    let _top = map_toplevel(&mut fixture, app, &base, (400, 300));
    let window = windows(&fixture)[0];
    let (id, shell) = shell_client(&mut fixture);
    let before = fixture.model().windows()[&window].state.clone();
    shell.window_menu_pick("w999", WindowEntry::Left);
    shell.window_menu_pick("not-ours", WindowEntry::Fill);
    fixture.pump();
    assert!(fixture.client(id).protocol_error().is_none());
    assert_eq!(fixture.model().windows()[&window].state, before);
}

/// Two ordinary windows are open and the shell picks Left of Screen for the first: the shell
/// hears the choice (the first window, its half) and the other window as the one candidate; a
/// pick makes both halves, full screen on one Space, and the offer is taken away.
#[test]
fn left_of_screen_offers_the_other_window_and_a_pick_makes_both_halves() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let app = fixture.connect(ConnectionRole::Public);
    let base = WmBase::bind(fixture.client(app), 7);
    let _first = map_toplevel(&mut fixture, app, &base, (400, 300));
    let _second = map_toplevel(&mut fixture, app, &base, (300, 200));
    let (id, shell) = shell_client(&mut fixture);
    let (first, second) = match windows(&fixture)[..] {
        [first, second] => (first, second),
        ref other => panic!("two windows, got {other:?}"),
    };

    shell.window_menu_pick(&identifier_of(first), WindowEntry::TileLeftOfScreen);
    fixture.pump();
    assert_eq!(
        shell.split_choices(),
        [SplitChoiceSeen {
            first: identifier_of(first),
            side: Some(SplitSide::Left),
            candidates: vec![identifier_of(second)],
        }]
    );
    assert!(shell.split_ends().is_empty(), "the offer is still open");

    shell.split_pick(&identifier_of(second));
    fixture.pump();
    let spaces = |window: WindowId| match &fixture.model().windows()[&window].state {
        WindowState::FullScreen { space, .. } => Some(space.clone()),
        _ => None,
    };
    assert!(spaces(first).is_some(), "the first half is full screen");
    assert_eq!(spaces(first), spaces(second), "both halves share a Space");
    assert_eq!(shell.split_ends(), [identifier_of(first)]);
    assert!(fixture.client(id).protocol_error().is_none());
}

/// Escape or a click elsewhere (`split_cancel`) leaves the first window alone, full screen, and
/// takes the offer away; a pick for a window that is gone or unknown changes nothing.
#[test]
fn a_cancelled_choice_leaves_the_first_window_alone() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let app = fixture.connect(ConnectionRole::Public);
    let base = WmBase::bind(fixture.client(app), 7);
    let _first = map_toplevel(&mut fixture, app, &base, (400, 300));
    let _second = map_toplevel(&mut fixture, app, &base, (300, 200));
    let (id, shell) = shell_client(&mut fixture);
    let (first, second) = match windows(&fixture)[..] {
        [first, second] => (first, second),
        ref other => panic!("two windows, got {other:?}"),
    };

    shell.window_menu_pick(&identifier_of(first), WindowEntry::TileRightOfScreen);
    fixture.pump();
    assert_eq!(shell.split_choices().len(), 1);
    shell.split_pick("w999");
    shell.split_cancel();
    fixture.pump();
    assert_eq!(shell.split_ends(), [identifier_of(first)]);
    let state = |window: WindowId| fixture.model().windows()[&window].state.clone();
    assert!(matches!(state(first), WindowState::FullScreen { .. }));
    assert!(!matches!(state(second), WindowState::FullScreen { .. }));
    assert!(fixture.client(id).protocol_error().is_none());
}

/// Two windows share a Split View: Left of Screen for the first, a pick of the second.
fn split_pair(fixture: &mut Fixture) -> (WindowId, WindowId, ShellHandle, Toplevel) {
    let app = fixture.connect(ConnectionRole::Public);
    let base = WmBase::bind(fixture.client(app), 7);
    let _first = map_toplevel(fixture, app, &base, (400, 300));
    let second_top = map_toplevel(fixture, app, &base, (300, 200));
    let (_, shell) = shell_client(fixture);
    let (first, second) = match windows(fixture)[..] {
        [first, second] => (first, second),
        ref other => panic!("two windows, got {other:?}"),
    };
    shell.window_menu_pick(&identifier_of(first), WindowEntry::TileLeftOfScreen);
    fixture.pump();
    shell.split_pick(&identifier_of(second));
    fixture.pump();
    settle(fixture);
    (first, second, shell, second_top)
}

/// Whether the model holds a Split View Space now.
fn has_split_view(fixture: &Fixture) -> bool {
    fixture
        .model()
        .spaces()
        .iter()
        .any(|space| matches!(space.content, SpaceContent::SplitView(_)))
}

/// cas-wave "Real-run checks: Split View": Exit Full Screen on one half returns both windows to
/// where they were and ends the Split View; closing one half leaves the other full screen.
#[test]
fn exit_full_screen_returns_both_halves_and_closing_one_leaves_the_other_full_screen() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let (first, second, shell, _top) = split_pair(&mut fixture);
    assert!(has_split_view(&fixture), "the pair is a Split View");
    shell.window_menu_pick(&identifier_of(first), WindowEntry::ExitFullScreen);
    fixture.pump();
    settle(&mut fixture);
    let full = |fixture: &Fixture, window: WindowId| {
        matches!(
            fixture.model().windows()[&window].state,
            WindowState::FullScreen { .. }
        )
    };
    assert!(
        !full(&fixture, first) && !full(&fixture, second),
        "both back"
    );
    assert!(!has_split_view(&fixture), "the Split View is gone");

    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let (first, second, _shell, second_top) = split_pair(&mut fixture);
    second_top.destroy();
    fixture.pump();
    settle(&mut fixture);
    assert!(
        !fixture.model().windows().contains_key(&second),
        "the closed window is gone"
    );
    assert!(full(&fixture, first), "the other half stays full screen");
}
