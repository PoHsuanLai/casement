//! `xdg_shell` (P0-H2): bind every version, create/configure/ack/map, floating placement,
//! press-proved grabs, click to focus, no tiled states, popups, every documented error the
//! module's requests can raise, destruction order, and the transcript golden.
//!
//! Every test drives the real `State` through the `Fixture` and is ignored until the fixture
//! (P0-B-headless) and the core loop (P0-B-loop) are filled: they could not be run in this lane.

use casement_core::app::AppLife;
use casement_core::geometry::{LogicalPoint, LogicalRect, PhysicalSize};
use casement_core::input::{ButtonCode, DeviceClass, Edge, KeyCode};
use casement_core::policy::ConnectionRole;
use casement_core::window::WindowState;
use casement_core::{DeviceId, WindowId};
use casement_layout::{LayoutKnobs, RustScene, SceneLayout, spec_of};
use casement_testclient::surface::popup::{Popup, PopupSpec};
use casement_testclient::surface::toplevel::shell::{xdg_surface, xdg_wm_base};
use casement_testclient::surface::toplevel::{Pings, State, Toplevel, WmBase, bind_pointer};
use casement_testclient::{ClientError, ClientState, Pattern, shm_buffer};
use casement_testkit::{ClientId, Env, Fixture, FixtureSpec, TestOrigin};
use std::path::PathBuf;
use std::time::Duration;

/// Whether the golden files are replaced (`CASEMENT_BLESS=1`, deliberate).
pub(crate) fn blessing() -> bool {
    Env::of_process().bless == casement_render::golden::Bless::Yes
}

/// The path of a transcript golden.
pub(crate) fn golden(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden/transcripts")
        .join(name)
}

/// A started desktop and one connected public client, pumped.
pub(crate) fn boot() -> (Fixture, ClientId) {
    boot_as(ConnectionRole::Public)
}

/// A started desktop and one client connected through a socket of `role`, pumped.
pub(crate) fn boot_as(role: ConnectionRole) -> (Fixture, ClientId) {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let id = fixture.connect(role);
    fixture.pump();
    (fixture, id)
}

/// Creates a toplevel on `base`, takes the initial configure, acks it and shows a buffer of
/// `size`: the window maps.
pub(crate) fn map_toplevel(
    fixture: &mut Fixture,
    id: ClientId,
    base: &WmBase,
    size: (u32, u32),
) -> Toplevel {
    map_named(fixture, id, base, size, None)
}

/// As [`map_toplevel`], declaring `app_id` before the first commit.
pub(crate) fn map_named(
    fixture: &mut Fixture,
    id: ClientId,
    base: &WmBase,
    size: (u32, u32),
    app_id: Option<&str>,
) -> Toplevel {
    let top = base.toplevel(fixture.client(id));
    if let Some(app_id) = app_id {
        top.set_app_id(app_id);
    }
    top.commit_bare();
    fixture.pump();
    assert!(top.ack_latest().is_some(), "the initial configure arrives");
    let client = fixture.client(id);
    let buffer = shm_buffer(
        client,
        PhysicalSize {
            w: size.0,
            h: size.1,
        },
        Pattern::Gradient,
    );
    top.show(&buffer);
    fixture.pump();
    top
}

/// The model's windows, by id.
pub(crate) fn windows(fixture: &Fixture) -> Vec<WindowId> {
    fixture.model().windows().keys().copied().collect()
}

/// Where the layout puts `window` now (the same arrangement the compositor draws).
pub(crate) fn rect_of(fixture: &Fixture, window: WindowId) -> LogicalRect {
    let model = fixture.model();
    RustScene
        .arrange(&spec_of(model), &LayoutKnobs::of(model.settings()))
        .windows
        .iter()
        .find(|target| target.window == window)
        .map(|target| target.rect)
        .expect("the window is arranged")
}

/// Plugs a physical pointer.
pub(crate) fn pointer(fixture: &mut Fixture) -> DeviceId {
    fixture.input().plug(
        DeviceClass::Pointer,
        TestOrigin::Physical(DeviceClass::Pointer),
    )
}

/// Moves the pointer to the middle of `window` and clicks the left button.
pub(crate) fn click(fixture: &mut Fixture, device: DeviceId, window: WindowId) {
    let at = rect_of(fixture, window).centre();
    fixture.input().pointer_to(device, at);
    fixture
        .input()
        .button(device, ButtonCode(0x110), Edge::Press);
    fixture
        .input()
        .button(device, ButtonCode(0x110), Edge::Release);
}

/// Lets the open animations run out (windows are drawn, and hit, where they are at the clock's
/// now, so a click at a window's resting corner needs the window to have settled).
pub(crate) fn settle(fixture: &mut Fixture) {
    fixture.advance(std::time::Duration::from_secs(1));
}

/// Clicks the left button with the pointer at `at`.
pub(crate) fn click_at(fixture: &mut Fixture, device: DeviceId, at: LogicalPoint) {
    fixture.input().pointer_to(device, at);
    fixture
        .input()
        .button(device, ButtonCode(0x110), Edge::Press);
    fixture
        .input()
        .button(device, ButtonCode(0x110), Edge::Release);
}

/// Makes `window` the key window the way a person does: a click on it (a window mapped without a
/// launch token is not key, and only the key window may ask for full screen).
pub(crate) fn make_key(fixture: &mut Fixture, window: WindowId) {
    let device = pointer(fixture);
    settle(fixture);
    let at = corner_of(fixture, window, Corner::TopLeft);
    click_at(fixture, device, at);
    fixture.pump();
    assert_eq!(
        fixture.model().focus().key,
        Some(window),
        "the click made it key"
    );
}

/// A point of `window` that a window cascaded one step down and right of it does not cover (its
/// top-left corner, inset), and its mirror: a point of the cascaded window that the first does
/// not cover once it is raised (its bottom-right corner, inset).
pub(crate) fn corner_of(fixture: &Fixture, window: WindowId, corner: Corner) -> LogicalPoint {
    let rect = rect_of(fixture, window);
    let inset = 6_i32;
    match corner {
        Corner::TopLeft => LogicalPoint {
            x: rect.origin.x + inset,
            y: rect.origin.y + inset,
        },
        Corner::BottomRight => LogicalPoint {
            x: rect.origin.x + i32::try_from(rect.size.w).unwrap_or(0) - inset,
            y: rect.origin.y + i32::try_from(rect.size.h).unwrap_or(0) - inset,
        },
    }
}

/// Which corner [`corner_of`] means.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Corner {
    /// The top-left corner.
    TopLeft,
    /// The bottom-right corner.
    BottomRight,
}

/// Compares `actual` with the golden at `path`; with `CASEMENT_BLESS=1` the golden is replaced
/// (deliberate, with a reason in the commit).
pub(crate) fn check_golden(path: &std::path::Path, actual: &str) -> Result<(), String> {
    let golden = std::fs::read_to_string(path).unwrap_or_default();
    if golden == actual {
        return Ok(());
    }
    if blessing() {
        return std::fs::write(path, actual).map_err(|error| error.to_string());
    }
    Err(format!(
        "{} differs\n--- golden ---\n{golden}--- actual ---\n{actual}",
        path.display()
    ))
}

/// The transcript text of a client's flow, restricted to objects whose names start with one of
/// `prefixes`: one `object.event(args)` line per event, without `wm_capabilities` (smithay takes
/// its order from a `HashSet`, QUIRKS) and with the configure serials masked (the counter is
/// the process's).
pub(crate) fn lines_of(fixture: &mut Fixture, id: ClientId, prefixes: &[&str]) -> String {
    fixture
        .client(id)
        .transcript()
        .0
        .iter()
        .filter(|line| {
            prefixes
                .iter()
                .any(|prefix| line.object.starts_with(prefix))
                && line.event != "wm_capabilities"
        })
        .map(|line| {
            let args = match (line.object.starts_with("xdg_surface#"), line.event.as_str()) {
                (true, "configure") => "<serial>".to_owned(),
                _ => line.args.join(", "),
            };
            format!("{}.{}({args})\n", line.object, line.event)
        })
        .collect()
}

/// The transcript text of a client's xdg flow ([`lines_of`] over the `xdg_*` objects).
pub(crate) fn flow_text(fixture: &mut Fixture, id: ClientId) -> String {
    lines_of(fixture, id, &["xdg_"])
}

/// Xdg_wm_base binds at every version from 1 to 7; asking for 8 is refused by the client side.
#[test]
fn every_version_binds() {
    let (mut fixture, _) = boot();
    for version in 1..=7 {
        let id = fixture.connect(ConnectionRole::Public);
        fixture.pump();
        let base = WmBase::bind(fixture.client(id), version);
        let top = base.toplevel(fixture.client(id));
        top.commit_bare();
        fixture.pump();
        assert!(
            fixture.client(id).protocol_error().is_none(),
            "version {version}"
        );
        // Every version sends the initial configure; wm_capabilities and bounds are version-gated.
        assert_eq!(top.configures().len(), 1, "version {version}");
    }
    let id = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    assert!(matches!(
        fixture
            .client(id)
            .bind::<xdg_wm_base::XdgWmBase, _>(8..=8, Pings::default()),
        Err(ClientError::MissingGlobal(_))
    ));
}

/// The first window is centred in the usable area, the next one cascade step further (floating only).
#[test]
fn new_windows_centre_then_cascade() {
    let (mut fixture, id) = boot();
    let base = WmBase::bind(fixture.client(id), 7);
    map_toplevel(&mut fixture, id, &base, (400, 300));
    map_toplevel(&mut fixture, id, &base, (400, 300));
    let ids = windows(&fixture);
    assert_eq!(ids.len(), 2);
    let (first, second) = (rect_of(&fixture, ids[0]), rect_of(&fixture, ids[1]));
    // 1920x1080 at scale 1: the first window is centred.
    assert_eq!(first.origin, LogicalPoint { x: 760, y: 390 });
    assert_eq!((first.size.w, first.size.h), (400, 300));
    // The second is one cascade step down and to the right of the first, same size.
    assert!(second.origin.x > first.origin.x && second.origin.y > first.origin.y);
    assert_eq!(second.size, first.size);
}

/// Move/resize with a serial that answers no press does nothing; with a button serial they start a grab.
#[test]
fn move_and_resize_need_a_press_serial() {
    let (mut fixture, id) = boot();
    let base = WmBase::bind(fixture.client(id), 7);
    let pointer = bind_pointer(fixture.client(id));
    fixture.pump();
    let top = map_toplevel(&mut fixture, id, &base, (400, 300));
    let window = windows(&fixture)[0];
    let device = self::pointer(&mut fixture);
    // The first click activates the window and is swallowed (A5); the second is delivered.
    click(&mut fixture, device, window);
    click(&mut fixture, device, window);
    let serial = pointer.last_press().expect("a press was delivered");
    let before = rect_of(&fixture, window);

    // A serial no press answers: the request is ignored and the pointer drags nothing.
    top.request_move(&pointer.seat, serial.wrapping_add(1000));
    fixture.pump();
    let at = before.centre();
    fixture.input().pointer_to(
        device,
        LogicalPoint {
            x: at.x + 90,
            y: at.y + 40,
        },
    );
    assert_eq!(rect_of(&fixture, window), before);
    assert!(
        fixture.client(id).protocol_error().is_none(),
        "a bad serial is not a protocol error"
    );

    // A button serial: the grab starts and the window follows the pointer.
    fixture.input().pointer_to(device, at);
    top.request_move(&pointer.seat, serial);
    fixture.pump();
    fixture
        .input()
        .button(device, ButtonCode(0x110), Edge::Press);
    fixture.input().pointer_to(
        device,
        LogicalPoint {
            x: at.x + 90,
            y: at.y + 40,
        },
    );
    let after = rect_of(&fixture, window);
    assert_eq!(after.origin.x, before.origin.x + 90);
    assert_eq!(after.origin.y, before.origin.y + 40);
}

/// A press on an inactive window activates it and sends activated in a configure.
#[test]
fn click_to_focus() {
    let (mut fixture, id) = boot();
    let base = WmBase::bind(fixture.client(id), 7);
    // Two windows of one size, the second cascaded: each has a corner the other does not cover
    // (a window cannot be clicked where another lies over it).
    let first = map_toplevel(&mut fixture, id, &base, (400, 300));
    let second = map_toplevel(&mut fixture, id, &base, (400, 300));
    // A window that maps with no token takes no focus (A7).
    assert_eq!(fixture.model().focus().key, None);
    assert!(!first.newest_has(State::Activated) && !second.newest_has(State::Activated));
    settle(&mut fixture);
    let ids = windows(&fixture);
    let device = pointer(&mut fixture);
    let on_first = corner_of(&fixture, ids[0], Corner::TopLeft);
    let on_second = corner_of(&fixture, ids[1], Corner::BottomRight);
    click_at(&mut fixture, device, on_first);
    assert_eq!(fixture.model().focus().key, Some(ids[0]));
    assert!(first.newest_has(State::Activated));
    assert!(!second.newest_has(State::Activated));
    click_at(&mut fixture, device, on_second);
    assert_eq!(fixture.model().focus().key, Some(ids[1]));
    assert!(second.newest_has(State::Activated));
    assert!(
        !first.newest_has(State::Activated),
        "the first is told it is inactive"
    );
}

/// P0 never sends tiled_* states.
#[test]
fn a_floating_configure_has_no_tiled_edges() {
    let (mut fixture, id) = boot();
    let base = WmBase::bind(fixture.client(id), 7);
    let top = map_toplevel(&mut fixture, id, &base, (400, 300));
    let device = pointer(&mut fixture);
    let window = windows(&fixture)[0];
    click(&mut fixture, device, window);
    top.set_maximized();
    fixture.pump();
    let tiled = [
        State::TiledTop,
        State::TiledBottom,
        State::TiledLeft,
        State::TiledRight,
    ];
    for (n, configure) in top.configures().iter().enumerate() {
        for state in tiled {
            assert!(
                !configure.states.contains(&state),
                "configure {n} has {state:?}"
            );
        }
    }
    assert!(
        top.newest_has(State::Maximized),
        "a zoom is `maximized`, never four tiled edges"
    );
}

/// Create, configure, ack, map, close: the transcript matches tests/golden/transcripts/xdg_shell-map.txt.
#[test]
fn toplevel_transcript_golden() {
    let (mut fixture, id) = boot();
    let base = WmBase::bind(fixture.client(id), 7);
    let top = map_toplevel(&mut fixture, id, &base, (400, 300));
    let device = pointer(&mut fixture);
    let window = windows(&fixture)[0];
    click(&mut fixture, device, window);
    top.toplevel.set_title("golden".to_owned());
    fixture.pump();
    // Closing is the compositor's ask; the client then destroys its objects in order.
    top.destroy();
    fixture.pump();
    let text = flow_text(&mut fixture, id);
    check_golden(&golden("xdg_shell-map.txt"), &text).expect("the toplevel flow");
}

/// Titles, app ids and limits a client declares before it maps reach the model with the window.
#[test]
fn what_a_client_declares_reaches_the_model() {
    let (mut fixture, id) = boot();
    let base = WmBase::bind(fixture.client(id), 7);
    let top = base.toplevel(fixture.client(id));
    top.set_title("Inbox");
    top.set_app_id("org.example.Mail");
    top.set_limits((200, 100), (800, 600));
    top.commit_bare();
    fixture.pump();
    top.ack_latest();
    let buffer = shm_buffer(
        fixture.client(id),
        PhysicalSize { w: 400, h: 300 },
        Pattern::Gradient,
    );
    top.show(&buffer);
    fixture.pump();
    let window = windows(&fixture)[0];
    let held = &fixture.model().windows()[&window];
    assert_eq!(held.title.0, "Inbox");
    assert_eq!(held.claimed_id.as_str(), "org.example.Mail");
    assert_eq!((held.limits.min.w, held.limits.min.h), (200, 100));
    // A later title is a change event, not a new window.
    top.set_title("Inbox (3)");
    fixture.pump();
    assert_eq!(fixture.model().windows().len(), 1);
    assert_eq!(fixture.model().windows()[&window].title.0, "Inbox (3)");
}

/// A null buffer unmaps the window; the same toplevel maps again after a new configure.
#[test]
fn a_null_buffer_unmaps_and_the_next_buffer_maps_again() {
    let (mut fixture, id) = boot();
    let base = WmBase::bind(fixture.client(id), 7);
    let top = map_toplevel(&mut fixture, id, &base, (400, 300));
    assert_eq!(fixture.model().windows().len(), 1);
    top.surface.attach(None, 0, 0);
    top.surface.commit();
    fixture.pump();
    assert!(fixture.model().windows().is_empty());
    top.commit_bare();
    fixture.pump();
    assert!(
        top.ack_latest().is_some(),
        "an unmapped toplevel is configured again"
    );
    let buffer = shm_buffer(
        fixture.client(id),
        PhysicalSize { w: 400, h: 300 },
        Pattern::Gradient,
    );
    top.show(&buffer);
    fixture.pump();
    assert_eq!(fixture.model().windows().len(), 1);
}

/// A popup is configured at its first commit, placed inside the output (it slides back from an
/// edge), repositioned with its token, and dismissed when its grab answers no press.
#[test]
fn a_popup_is_placed_inside_the_output_and_repositioned() {
    let (mut fixture, id) = boot();
    let base = WmBase::bind(fixture.client(id), 7);
    let parent = map_toplevel(&mut fixture, id, &base, (400, 300));
    let surface = fixture.client(id).create_surface().expect("a surface");
    let popup = Popup::of(
        fixture.client(id),
        &base,
        &parent,
        surface,
        PopupSpec::menu_at((380, 10)),
    );
    popup.surface.commit();
    fixture.pump();
    let (x, y, w, h) = popup.geometry().expect("the initial popup configure");
    assert_eq!((w, h), (100, 50));
    // The anchor is at the parent's right edge; the popup hangs from it inside the output.
    assert_eq!((x, y), (380, 10));
    popup.reposition(fixture.client(id), &base, PopupSpec::menu_at((20, 20)), 7);
    fixture.pump();
    assert_eq!(popup.log.with(|seen| seen.repositioned.clone()), vec![7]);
    assert_eq!(popup.geometry(), Some((20, 20, 100, 50)));
    // A grab with a serial that answers no press is refused: the popup is dismissed. The
    // protocol puts `grab` before the popup's first commit (xdg_popup.invalid_grab otherwise
    // once it is mapped), and smithay hands it to the compositor at that commit, so this one
    // is a second popup that grabs first and commits after.
    let pointer = bind_pointer(fixture.client(id));
    fixture.pump();
    let surface = fixture.client(id).create_surface().expect("a surface");
    let refused = Popup::of(
        fixture.client(id),
        &base,
        &parent,
        surface,
        PopupSpec::menu_at((20, 20)),
    );
    refused.grab(&pointer.seat, 9999);
    refused.surface.commit();
    fixture.pump();
    assert_eq!(refused.log.with(|seen| seen.done), 1);
    assert_eq!(popup.log.with(|seen| seen.done), 0, "only the refused one");
    assert!(fixture.client(id).protocol_error().is_none());
}

/// Destroying the `xdg_wm_base` while an `xdg_surface` lives is `defunct_surfaces`.
#[test]
fn destroying_the_base_first_is_defunct_surfaces() {
    let (mut fixture, id) = boot();
    let base = WmBase::bind(fixture.client(id), 7);
    let _top = base.toplevel(fixture.client(id));
    base.base.destroy();
    fixture.pump();
    let error = fixture
        .client(id)
        .protocol_error()
        .expect("a protocol error");
    assert_eq!(error.interface, "xdg_wm_base");
    assert_eq!(error.code, xdg_wm_base::Error::DefunctSurfaces as u32);
    // Only that client is disconnected.
    let other = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    assert!(fixture.client(other).protocol_error().is_none());
}

/// An `xdg_surface` destroyed before its `xdg_toplevel` is `xdg_surface.defunct_role_object`;
/// the second `get_toplevel` on one surface is `xdg_surface.already_constructed` (xdg-shell.xml).
/// Upstream smithay (118e34ff, `shell/xdg/handlers/surface.rs`) posts `xdg_wm_base.role` for the
/// first and does not refuse the second (`give_role` with the same role succeeds, so a surface
/// got two toplevels). Both are decided inside smithay's request handler, where the compositor
/// has no hook, so the casement fork carries the fix (QUIRKS row).
#[test]
fn destruction_order_and_double_roles_are_errors() {
    let (mut fixture, id) = boot();
    let base = WmBase::bind(fixture.client(id), 7);
    let top = base.toplevel(fixture.client(id));
    top.xdg_surface.destroy();
    fixture.pump();
    let error = fixture
        .client(id)
        .protocol_error()
        .expect("a protocol error");
    assert_eq!(error.interface, "xdg_surface");
    assert_eq!(error.code, xdg_surface::Error::DefunctRoleObject as u32);

    let id = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let base = WmBase::bind(fixture.client(id), 7);
    let top = base.toplevel(fixture.client(id));
    let _second = top
        .xdg_surface
        .get_toplevel(&fixture.client(id).qh(), top.log.clone());
    fixture.pump();
    let error = fixture
        .client(id)
        .protocol_error()
        .expect("a protocol error");
    assert_eq!(error.interface, "xdg_surface");
    assert_eq!(error.code, xdg_surface::Error::AlreadyConstructed as u32);
}

/// Destroying an `xdg_surface` before its role object disconnects that client and only that
/// one (which error it gets is the test above's business).
#[test]
fn destroying_the_surface_before_its_role_disconnects_only_that_client() {
    let (mut fixture, id) = boot();
    let bystander = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let base = WmBase::bind(fixture.client(id), 7);
    let top = base.toplevel(fixture.client(id));
    top.xdg_surface.destroy();
    fixture.pump();
    assert!(fixture.client(id).protocol_error().is_some());
    assert!(!fixture.client(id).is_connected());
    assert!(fixture.client(bystander).is_connected());
}

/// An `xdg_wm_base` that hears pings and never answers: a client whose main loop is stuck.
#[derive(Debug)]
struct Deaf;

impl wayland_client::Dispatch<xdg_wm_base::XdgWmBase, Deaf> for ClientState {
    fn event(
        _: &mut Self,
        _: &xdg_wm_base::XdgWmBase,
        _: xdg_wm_base::Event,
        _: &Deaf,
        _: &wayland_client::Connection,
        _: &wayland_client::QueueHandle<Self>,
    ) {
    }
}

/// Typing into a window whose client stops answering pings marks its app not responding once
/// the deadline passes (P3-QU); typing with an answering client does not.
#[test]
fn a_client_that_stops_answering_pings_is_not_responding() {
    let (mut fixture, id) = boot();
    let base = fixture
        .client(id)
        .bind::<xdg_wm_base::XdgWmBase, _>(1..=7, Deaf)
        .expect("xdg_wm_base");
    let base = WmBase {
        base,
        pings: Pings::default(),
    };
    map_toplevel(&mut fixture, id, &base, (400, 300));
    let (window, app) = fixture
        .model()
        .windows()
        .iter()
        .next()
        .map(|(id, window)| (*id, window.app.clone()))
        .expect("a window");
    fixture.activate_window(window);
    let keyboard = fixture.input().plug(
        DeviceClass::Keyboard,
        TestOrigin::Physical(DeviceClass::Keyboard),
    );
    fixture.input().key(keyboard, KeyCode(30), Edge::Press);
    fixture.advance(Duration::from_secs(1));
    assert_eq!(fixture.model().apps()[&app].life, AppLife::Running);
    fixture.advance(Duration::from_secs(5));
    assert!(matches!(
        fixture.model().apps()[&app].life,
        AppLife::NotResponding { .. }
    ));
}

/// The same typing with a client that answers leaves its app running.
#[test]
fn a_client_that_answers_pings_stays_responding() {
    let (mut fixture, id) = boot();
    let base = WmBase::bind(fixture.client(id), 7);
    map_toplevel(&mut fixture, id, &base, (400, 300));
    let (window, app) = fixture
        .model()
        .windows()
        .iter()
        .next()
        .map(|(id, window)| (*id, window.app.clone()))
        .expect("a window");
    fixture.activate_window(window);
    let keyboard = fixture.input().plug(
        DeviceClass::Keyboard,
        TestOrigin::Physical(DeviceClass::Keyboard),
    );
    fixture.input().key(keyboard, KeyCode(30), Edge::Press);
    fixture.advance(Duration::from_secs(6));
    assert_eq!(fixture.model().apps()[&app].life, AppLife::Running);
    assert!(!base.pings.with(|seen| seen.is_empty()), "it was pinged");
}

/// CASC-03: a client may hold `MAX_TOPLEVELS` toplevels; the next one cuts it with
/// `wl_display.no_memory` (code 2), so a loop of `get_toplevel` cannot grow the model without
/// end. Another client is not affected.
#[test]
fn a_client_that_makes_toplevels_without_end_is_cut() {
    let (mut fixture, id) = boot();
    let base = WmBase::bind(fixture.client(id), 7);
    let mut made = Vec::new();
    for _ in 0..casement_wayland::limits::MAX_TOPLEVELS {
        made.push(base.toplevel(fixture.client(id)));
    }
    fixture.pump();
    assert!(
        fixture.client(id).protocol_error().is_none(),
        "up to the cap is allowed"
    );
    made.push(base.toplevel(fixture.client(id)));
    fixture.pump();
    // The cut closes the connection (the system backend sends no error first).
    assert!(
        !fixture.client(id).is_connected(),
        "the client past the cap is cut"
    );
    let other = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    assert!(fixture.client(other).protocol_error().is_none());
}

/// The value of every `key=value` field of a timing line, by key.
fn timing_fields(line: &str) -> std::collections::BTreeMap<&str, &str> {
    line.split_whitespace()
        .skip(1)
        .filter_map(|field| field.split_once('='))
        .collect()
}

/// The nanosecond fields `keys` of a timing line, in that order.
fn stamps(line: &str, keys: &[&str]) -> Vec<u64> {
    let fields = timing_fields(line);
    keys.iter()
        .map(|key| fields[key].parse().expect("a number"))
        .collect()
}

/// The timing lines (workflow spec): a toplevel that maps writes one `window-ready` line, once,
/// with its stamps in order and the app id as the client gave it (whitespace as `_`); the click
/// that makes it key writes one `key-window` line whose input came before the change and the
/// change before the frame that showed it. Later frames write nothing more.
#[test]
fn a_window_and_a_click_write_their_timing_lines() {
    let (mut fixture, id) = boot();
    let base = WmBase::bind(fixture.client(id), 6);
    let top = map_named(
        &mut fixture,
        id,
        &base,
        (320, 200),
        Some("org.example.timing app"),
    );
    let window = *windows(&fixture).first().expect("a window");
    settle(&mut fixture);
    let ready: Vec<String> = fixture
        .timing_lines()
        .iter()
        .filter(|line| line.starts_with("window-ready "))
        .cloned()
        .collect();
    assert_eq!(ready.len(), 1, "{ready:?}");
    assert_eq!(timing_fields(&ready[0])["app_id"], "org.example.timing_app");
    let times = stamps(
        &ready[0],
        &[
            "connected_at",
            "first_commit_at",
            "first_frame_at",
            "settled_at",
        ],
    );
    assert!(times.windows(2).all(|pair| pair[0] <= pair[1]), "{times:?}");
    assert!(times[2] > 0, "a frame was presented: {times:?}");
    assert!(
        fixture
            .timing_lines()
            .iter()
            .all(|line| !line.starts_with("key-window "))
    );

    make_key(&mut fixture, window);
    fixture.advance(Duration::from_millis(100));
    settle(&mut fixture);
    let keys: Vec<String> = fixture
        .timing_lines()
        .iter()
        .filter(|line| line.starts_with("key-window "))
        .cloned()
        .collect();
    assert_eq!(keys.len(), 1, "{keys:?}");
    assert_eq!(timing_fields(&keys[0])["cause"], "pointer");
    let times = stamps(&keys[0], &["input_at", "focused_at", "presented_at"]);
    assert!(times[0] > 0, "the click is the input: {times:?}");
    assert!(times.windows(2).all(|pair| pair[0] <= pair[1]), "{times:?}");
    let count = fixture.timing_lines().len();
    fixture.advance(Duration::from_secs(1));
    assert_eq!(
        fixture.timing_lines().len(),
        count,
        "idle frames write nothing"
    );
    drop(top);
}

/// A window that draws its own frame (no decoration object, as Firefox and Zen) and is key goes
/// full screen when it asks: it takes a Space of its own, which is shown (the trial login of
/// 2026-10-19, where F11 in Zen left it where it was).
#[test]
fn a_key_window_with_its_own_frame_goes_full_screen_when_it_asks() {
    let (mut fixture, id) = boot();
    let base = WmBase::bind(fixture.client(id), 6);
    let top = map_toplevel(&mut fixture, id, &base, (800, 600));
    let window = windows(&fixture)[0];
    make_key(&mut fixture, window);
    let shown = |fixture: &Fixture| {
        fixture
            .model()
            .spaces()
            .shown()
            .map(|(_, space)| space.clone())
            .collect::<Vec<_>>()
    };
    let before = shown(&fixture);
    top.set_fullscreen();
    fixture.pump();
    let state = fixture.model().windows()[&window].state.clone();
    let WindowState::FullScreen { space, .. } = state else {
        panic!("full screen, not {state:?}");
    };
    assert!(!before.contains(&space), "a Space of its own");
    assert!(shown(&fixture).contains(&space), "and it is shown");
}
