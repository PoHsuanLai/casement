//! `quire_shell` (P2-Q2): the shell's surface roles, minimise targets and the events only the
//! compositor sees. Shell connections only.

use crate::protocol::xdg_shell::{map_toplevel, pointer, rect_of, settle, windows};
use casement_core::geometry::PhysicalSize;
use casement_core::input::{DeviceClass, Edge, KeyCode};
use casement_core::policy::ConnectionRole;
use casement_core::window::{Light, MenuEntry, WindowState, menu_point};
use casement_ipc::dbus::ConnectRefusal;
use casement_layout::{GroupHover, Scheme, frame_style};
use casement_protocols::vocab::ShellRole as ModelRole;
use casement_testclient::misc::decoration::DecorationManager;
use casement_testclient::quire::shell::{ShellError, ShellHandle, ShellRole, WindowEntry};
use casement_testclient::surface::layer::{
    Anchor, KeyboardInteractivity, Layer, LayerAsk, LayerSurface,
};
use casement_testclient::surface::toplevel::WmBase;
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
fn shell_client(fixture: &mut Fixture) -> (ClientId, ShellHandle) {
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
    let content = rect_of(&fixture, window);
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
        !on(MenuEntry::TileLeftOfScreen),
        "Split View is not built: dimmed"
    );

    shell.window_menu_pick(&identifier_of(window), WindowEntry::Left);
    fixture.pump();
    assert!(matches!(
        fixture.model().windows()[&window].state,
        WindowState::Tiled { .. }
    ));
    assert!(fixture.client(app).protocol_error().is_none());
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
