//! Edge reveal (owner OK 2026-10-22): in full screen the pointer resting at the top edge brings
//! the surface with the role `bar` over the window, and at the bottom edge the `dock`'s, each
//! after its own delay; the shell is told with `reveal` (`reveal_on_output` from version 7);
//! a revealed surface takes the pointer over the window; moving off puts it away; the window is
//! never reconfigured. Outside full screen nothing happens.
//!
//! Version 7: the shell chooses to keep the bar up in full screen (`set_full_screen_bar`: the
//! window is sized below the bar and the top edge reveals nothing), says the person wants reduced
//! motion (`set_motion`: a quicker cross-fade), and a bar and a dock are held once per output.

use super::layer_shell::{map, map_on};
use crate::protocol::xdg_shell::{make_key, map_toplevel, pointer, settle, windows};
use casement_core::DeviceId;
use casement_core::geometry::{LogicalPoint, PhysicalSize};
use casement_core::policy::ConnectionRole;
use casement_testclient::quire::shell::{
    BoundOutput, FullScreenBar, Motion, ScreenEdge, ShellError, ShellHandle, ShellRole, Shown,
};
use casement_testclient::seat::pointer::Pointer;
use casement_testclient::surface::layer::{
    Anchor, KeyboardInteractivity, Layer, LayerAsk, LayerSurface,
};
use casement_testclient::surface::subsurface::bind_outputs as app_bind_outputs;
use casement_testclient::surface::toplevel::{Toplevel, WmBase};
use casement_testclient::{Pattern, TestClient, shm_buffer};
use casement_testkit::{ClientId, Fixture, FixtureSpec};
use std::time::Duration;

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

/// The shell's bar: along the top, reserving room.
fn bar_ask() -> LayerAsk {
    LayerAsk {
        level: Layer::Top,
        namespace: "bar",
        anchor: Anchor::Top | Anchor::Left | Anchor::Right,
        exclusive_zone: 32,
        keyboard: KeyboardInteractivity::None,
        size: (0, 32),
    }
}

/// The shell's dock: centred along the bottom.
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

/// A desktop with a window, a shell with a bar and a dock, and the window gone full screen.
struct Desk {
    fixture: Fixture,
    mouse: DeviceId,
    shell_id: ClientId,
    shell: ShellHandle,
    outputs: Vec<BoundOutput>,
    window: Toplevel,
    _bar: LayerSurface,
    _dock: LayerSurface,
}

/// Starts the desk with a shell that bound version 6 (it hears the plain `reveal`); the window is
/// full screen when `full` is true, else it floats.
fn desk(full: bool) -> Desk {
    desk_at(full, 6)
}

/// As [`desk`], with a shell that bound `version`.
fn desk_at(full: bool, version: u32) -> Desk {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let mouse = pointer(&mut fixture);
    let app = fixture.connect(ConnectionRole::Public);
    let base = WmBase::bind(fixture.client(app), 7);
    let window = map_toplevel(&mut fixture, app, &base, (400, 300));
    let shell_id = fixture.connect(ConnectionRole::Shell);
    let shell = ShellHandle::bind(fixture.client(shell_id), version);
    let outputs = ShellHandle::bind_outputs(fixture.client(shell_id));
    fixture.pump();
    let bar = map(&mut fixture, shell_id, &bar_ask());
    shell.set_role(bar.surface(), ShellRole::Bar);
    let dock = map(&mut fixture, shell_id, &dock_ask());
    shell.set_role(dock.surface(), ShellRole::Dock);
    let _pointer = Pointer::bind(fixture.client(shell_id)).expect("a pointer");
    fixture.pump();
    if full {
        let id = windows(&fixture)[0];
        make_key(&mut fixture, id);
        window.set_fullscreen();
        fixture.pump();
        assert!(window.ack_latest().is_some());
        let buffer = shm_buffer(
            fixture.client(app),
            PhysicalSize { w: 1920, h: 1080 },
            Pattern::Gradient,
        );
        window.show(&buffer);
        fixture.pump();
        settle(&mut fixture);
    }
    Desk {
        fixture,
        mouse,
        shell_id,
        shell,
        outputs,
        window,
        _bar: bar,
        _dock: dock,
    }
}

/// How many times the shell's pointer was told it entered a surface.
fn entered(client: &TestClient) -> usize {
    client
        .transcript()
        .0
        .iter()
        .filter(|line| line.object.starts_with("wl_pointer#") && line.event == "enter")
        .count()
}

fn told(edge: ScreenEdge, shown: Shown) -> (Option<ScreenEdge>, Option<Shown>) {
    (Some(edge), Some(shown))
}

/// Each edge shows after its own rest and not a millisecond before, and goes when the pointer
/// moves off: the bar after 450 ms, the dock after the 200 ms of its auto-hide.
#[test]
fn the_bar_and_the_dock_come_after_their_own_rest_and_go_when_the_pointer_moves_off() {
    // name, the pointer's row at the edge, the edge, the rest in ms
    let cases = [
        ("bar", 1, ScreenEdge::Top, 450),
        ("dock", 1078, ScreenEdge::Bottom, 200),
    ];
    for (name, y, edge, rest) in cases {
        let mut desk = desk(true);
        let mouse = desk.mouse;
        let (fixture, shell) = (&mut desk.fixture, &desk.shell);
        fixture
            .input()
            .pointer_to(mouse, LogicalPoint { x: 960, y });
        fixture.advance(ms(rest - 1));
        assert!(shell.reveals().is_empty(), "{name}: not before {rest} ms");
        fixture.advance(ms(1));
        assert_eq!(shell.reveals(), [told(edge, Shown::Shown)], "{name}");

        // Resting on it keeps it; the middle of the screen is the window's, and leaving puts it
        // away: the shell hears `hidden` when the slide-away ends.
        fixture.advance(ms(1000));
        assert_eq!(
            shell.reveals().len(),
            1,
            "{name}: it stays while the pointer rests"
        );
        fixture
            .input()
            .pointer_to(mouse, LogicalPoint { x: 960, y: 540 });
        fixture.pump();
        assert_eq!(shell.reveals().len(), 1, "{name}: not while it slides away");
        fixture.advance(ms(1000));
        assert_eq!(
            shell.reveals(),
            [told(edge, Shown::Shown), told(edge, Shown::Hidden)],
            "{name}"
        );
        fixture.advance(ms(1000));
        assert_eq!(shell.reveals().len(), 2, "{name}: and says no more");
    }
}

/// A pointer that leaves the strip before the rest is long enough brings nothing.
#[test]
fn leaving_the_strip_early_cancels_the_reveal() {
    let mut desk = desk(true);
    let mouse = desk.mouse;
    let (fixture, shell) = (&mut desk.fixture, &desk.shell);
    fixture
        .input()
        .pointer_to(mouse, LogicalPoint { x: 960, y: 1 });
    fixture.advance(ms(300));
    fixture
        .input()
        .pointer_to(mouse, LogicalPoint { x: 960, y: 200 });
    fixture.advance(ms(1000));
    assert!(shell.reveals().is_empty());
}

/// The revealed bar is above the window for the pointer, the window is never reconfigured, and
/// once the bar has gone the pointer is the window's again.
#[test]
fn the_revealed_bar_takes_the_pointer_over_the_window_and_the_window_keeps_its_size() {
    let mut desk = desk(true);
    let mouse = desk.mouse;
    let configures = desk.window.configures().len();
    let id = desk.shell_id;
    let (fixture, shell) = (&mut desk.fixture, &desk.shell);
    fixture
        .input()
        .pointer_to(mouse, LogicalPoint { x: 960, y: 1 });
    fixture.advance(ms(450));
    assert_eq!(shell.reveals(), [told(ScreenEdge::Top, Shown::Shown)]);
    assert_eq!(
        entered(fixture.client(id)),
        0,
        "it starts wholly above the output, so the pointer is still the window's"
    );

    fixture.advance(ms(300));
    assert_eq!(
        entered(fixture.client(id)),
        1,
        "the bar is under the pointer and over the window"
    );

    fixture
        .input()
        .pointer_to(mouse, LogicalPoint { x: 960, y: 540 });
    fixture.advance(ms(300));
    assert_eq!(
        shell.reveals(),
        [
            told(ScreenEdge::Top, Shown::Shown),
            told(ScreenEdge::Top, Shown::Hidden)
        ]
    );
    fixture
        .input()
        .pointer_to(mouse, LogicalPoint { x: 960, y: 10 });
    assert_eq!(
        entered(fixture.client(id)),
        1,
        "under the window again, the bar takes no pointer"
    );
    assert_eq!(
        desk.window.configures().len(),
        configures,
        "the bar and the dock overlay the window: no configure"
    );
}

/// Outside full screen the edges do nothing: the bar is already where it belongs.
#[test]
fn outside_full_screen_the_edges_reveal_nothing() {
    let mut desk = desk(false);
    let mouse = desk.mouse;
    let (fixture, shell) = (&mut desk.fixture, &desk.shell);
    for y in [1, 1078] {
        fixture
            .input()
            .pointer_to(mouse, LogicalPoint { x: 960, y });
        fixture.advance(ms(2000));
    }
    assert!(shell.reveals().is_empty());
}

/// The edge and state of every `reveal_on_output` the shell heard, whichever output it named.
fn edges_of(shell: &ShellHandle) -> Vec<(Option<ScreenEdge>, Option<Shown>)> {
    shell
        .reveals_on_outputs()
        .into_iter()
        .map(|(_, edge, shown)| (edge, shown))
        .collect()
}

/// The size the window was last configured to.
fn configured(window: &Toplevel) -> Option<(i32, i32)> {
    window.configures().last().map(|configure| configure.size)
}

/// A shell that bound version 7 hears `reveal_on_output` naming its `wl_output` and no plain
/// `reveal`; one that bound an older version hears `reveal` and no `reveal_on_output`.
#[test]
fn a_version_7_shell_hears_the_reveal_on_its_output_in_place_of_reveal() {
    let mut desk = desk_at(true, 7);
    let mouse = desk.mouse;
    let output = desk.outputs[0].object.clone();
    let old_id = desk.fixture.connect(ConnectionRole::Shell);
    let old = ShellHandle::bind(desk.fixture.client(old_id), 6);
    let (fixture, shell) = (&mut desk.fixture, &desk.shell);
    fixture.pump();
    fixture
        .input()
        .pointer_to(mouse, LogicalPoint { x: 960, y: 1 });
    fixture.advance(ms(450));
    assert_eq!(
        shell.reveals_on_outputs(),
        [(output.clone(), Some(ScreenEdge::Top), Some(Shown::Shown))]
    );
    assert!(shell.reveals().is_empty(), "no plain reveal beside it");
    assert_eq!(old.reveals(), [told(ScreenEdge::Top, Shown::Shown)]);
    assert!(
        old.reveals_on_outputs().is_empty(),
        "version 6 hears the old one"
    );

    fixture
        .input()
        .pointer_to(mouse, LogicalPoint { x: 960, y: 540 });
    fixture.advance(ms(1000));
    assert_eq!(
        shell.reveals_on_outputs().last(),
        Some(&(output, Some(ScreenEdge::Top), Some(Shown::Hidden)))
    );
    assert!(shell.reveals().is_empty());
}

/// With `keep` the full-screen window is configured to the output less the bar's zone and the
/// top edge reveals nothing, while the dock still reveals; `reveal` sizes it back and the top edge
/// reveals again.
#[test]
fn keeping_the_bar_sizes_the_full_screen_window_below_it_and_reveals_nothing_at_the_top() {
    let mut desk = desk_at(true, 7);
    let mouse = desk.mouse;
    assert_eq!(
        configured(&desk.window),
        Some((1920, 1080)),
        "the whole output"
    );
    desk.shell.set_full_screen_bar(FullScreenBar::Keep);
    desk.fixture.pump();
    assert_eq!(
        configured(&desk.window),
        Some((1920, 1048)),
        "the output less the bar's 32 px"
    );
    assert!(desk.window.ack_latest().is_some());
    let (fixture, shell) = (&mut desk.fixture, &desk.shell);

    fixture
        .input()
        .pointer_to(mouse, LogicalPoint { x: 960, y: 1 });
    fixture.advance(ms(2000));
    assert!(
        shell.reveals_on_outputs().is_empty(),
        "the bar is already up"
    );

    fixture
        .input()
        .pointer_to(mouse, LogicalPoint { x: 960, y: 1078 });
    fixture.advance(ms(200));
    assert_eq!(
        edges_of(shell),
        [(Some(ScreenEdge::Bottom), Some(Shown::Shown))],
        "the dock still reveals"
    );

    shell.set_full_screen_bar(FullScreenBar::Reveal);
    fixture.pump();
    assert_eq!(
        configured(&desk.window),
        Some((1920, 1080)),
        "back to full size"
    );
    fixture
        .input()
        .pointer_to(mouse, LogicalPoint { x: 960, y: 1 });
    fixture.advance(ms(450));
    assert_eq!(
        edges_of(shell).last(),
        Some(&(Some(ScreenEdge::Top), Some(Shown::Shown))),
        "the top edge reveals again"
    );
}

/// `set_motion(reduced)` is accepted, and the bar then goes in the quick cross-fade (120 ms)
/// where it slid for 200 ms.
#[test]
fn reduced_motion_is_accepted_and_the_bar_goes_in_the_quick_fade() {
    let mut desk = desk_at(true, 7);
    let mouse = desk.mouse;
    let id = desk.shell_id;
    desk.shell.set_motion(Motion::Reduced);
    let (fixture, shell) = (&mut desk.fixture, &desk.shell);
    fixture.pump();
    assert!(fixture.client(id).protocol_error().is_none());
    fixture
        .input()
        .pointer_to(mouse, LogicalPoint { x: 960, y: 1 });
    fixture.advance(ms(450));
    assert_eq!(edges_of(shell).len(), 1, "the rest is as long as ever");
    fixture.advance(ms(1000));
    fixture
        .input()
        .pointer_to(mouse, LogicalPoint { x: 960, y: 540 });
    fixture.advance(ms(119));
    assert_eq!(edges_of(shell).len(), 1, "still fading out");
    fixture.advance(ms(1));
    assert_eq!(
        edges_of(shell).last(),
        Some(&(Some(ScreenEdge::Top), Some(Shown::Hidden))),
        "gone after the quick duration, not the slide's 200 ms"
    );
}

/// Two outputs, each with its own bar: no `role_taken`, and the reveal on the second names the
/// second output. A second bar on one output is `role_taken`.
#[test]
fn each_output_has_its_own_bar_and_its_reveal_names_that_output() {
    let spec = FixtureSpec::desktop().with_outputs(&[
        "HEADLESS-1:1920x1080@60000:120:0,0",
        "HEADLESS-2:1920x1080@60000:120:1920,0",
    ]);
    let mut fixture = Fixture::start(spec).expect("fixture");
    let mouse = pointer(&mut fixture);
    let app = fixture.connect(ConnectionRole::Public);
    let base = WmBase::bind(fixture.client(app), 7);
    let window = map_toplevel(&mut fixture, app, &base, (400, 300));
    fixture.pump();
    let app_outputs = app_bind_outputs(fixture.client(app), 4, true).expect("outputs bind");
    fixture.pump();
    let app_second = app_outputs
        .iter()
        .find(|one| one.facts(fixture.client(app)).name.as_deref() == Some("HEADLESS-2"))
        .map(|one| one.output.clone())
        .expect("the app's second output");
    let shell_id = fixture.connect(ConnectionRole::Shell);
    let shell = ShellHandle::bind(fixture.client(shell_id), 7);
    let outputs = ShellHandle::bind_outputs(fixture.client(shell_id));
    fixture.pump();
    let named = |name: &str| {
        outputs
            .iter()
            .find(|bound| bound.connector().as_deref() == Some(name))
            .map(|bound| bound.object.clone())
            .expect("the output")
    };
    let (first, second) = (named("HEADLESS-1"), named("HEADLESS-2"));
    let bar_one = map_on(&mut fixture, shell_id, &bar_ask(), Some(&first));
    shell.set_role(bar_one.surface(), ShellRole::Bar);
    let bar_two = map_on(&mut fixture, shell_id, &bar_ask(), Some(&second));
    shell.set_role(bar_two.surface(), ShellRole::Bar);
    fixture.pump();
    assert!(
        fixture.client(shell_id).protocol_error().is_none(),
        "one bar per output"
    );

    let id = windows(&fixture)[0];
    make_key(&mut fixture, id);
    // The app's own wl_output: an object never crosses connections.
    window.set_fullscreen_on(&app_second);
    fixture.pump();
    assert!(window.ack_latest().is_some());
    let buffer = shm_buffer(
        fixture.client(app),
        PhysicalSize { w: 1920, h: 1080 },
        Pattern::Gradient,
    );
    window.show(&buffer);
    fixture.pump();
    settle(&mut fixture);
    fixture
        .input()
        .pointer_to(mouse, LogicalPoint { x: 2880, y: 1 });
    fixture.advance(ms(450));
    assert_eq!(
        shell.reveals_on_outputs(),
        [(second, Some(ScreenEdge::Top), Some(Shown::Shown))],
        "the reveal names the second output"
    );

    let third = map_on(&mut fixture, shell_id, &bar_ask(), Some(&first));
    shell.set_role(third.surface(), ShellRole::Bar);
    fixture.pump();
    let error = fixture
        .client(shell_id)
        .protocol_error()
        .expect("a second bar on the first output is role_taken");
    assert_eq!(error.code, ShellError::RoleTaken as u32);
}
