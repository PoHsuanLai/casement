//! Edge reveal (owner OK 2026-10-22): in full screen the pointer resting at the top edge brings
//! the surface with the role `bar` over the window, and at the bottom edge the `dock`'s, each
//! after its own delay; the shell is told with `reveal`; a revealed surface takes the pointer
//! over the window; moving off puts it away; the window is never reconfigured. Outside full
//! screen nothing happens.

use super::layer_shell::map;
use super::quire_shell::shell_client;
use crate::protocol::xdg_shell::{make_key, map_toplevel, pointer, settle, windows};
use casement_core::DeviceId;
use casement_core::geometry::{LogicalPoint, PhysicalSize};
use casement_core::policy::ConnectionRole;
use casement_testclient::quire::shell::{ScreenEdge, ShellHandle, ShellRole, Shown};
use casement_testclient::seat::pointer::Pointer;
use casement_testclient::surface::layer::{
    Anchor, KeyboardInteractivity, Layer, LayerAsk, LayerSurface,
};
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
    window: Toplevel,
    _bar: LayerSurface,
    _dock: LayerSurface,
}

/// Starts the desk; the window is full screen when `full` is true, else it floats.
fn desk(full: bool) -> Desk {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let mouse = pointer(&mut fixture);
    let app = fixture.connect(ConnectionRole::Public);
    let base = WmBase::bind(fixture.client(app), 7);
    let window = map_toplevel(&mut fixture, app, &base, (400, 300));
    let (shell_id, shell) = shell_client(&mut fixture);
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
        // away at once.
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
