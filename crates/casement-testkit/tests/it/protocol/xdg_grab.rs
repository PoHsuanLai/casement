//! Interactive moves and resizes (`xdg_toplevel.move`, `xdg_toplevel.resize`): what ends a
//! grab, and what a resize may make of a window with size limits (P0-stubs).
//!
//! xdg-shell: the request answers a button press by its serial, and the grab lasts while that
//! button is held. A resize "respects" the window's declared minimum and maximum size: the
//! side the person did not grab does not move.

use crate::protocol::xdg_shell::{boot, click, pointer, rect_of, settle, windows};
use casement_core::DeviceId;
use casement_core::WindowId;
use casement_core::geometry::{LogicalPoint, LogicalRect, SubPoint, SubPx};
use casement_core::input::{ButtonCode, Edge};
use casement_core::window::WindowState;
use casement_layout::{LayoutKnobs, zone_rect};
use casement_protocols::vocab::TileZone;
use casement_testclient::surface::toplevel::{
    BoundPointer, ResizeEdge, Toplevel, WmBase, bind_pointer,
};
use casement_testkit::Fixture;

const BTN_LEFT: ButtonCode = ButtonCode(0x110);
const BTN_RIGHT: ButtonCode = ButtonCode(0x111);

/// A mapped window of `size` with the declared `limits` (min, max), the pointer and the press
/// serial of a delivered left click.
struct Desk {
    fixture: Fixture,
    top: Toplevel,
    seat: BoundPointer,
    device: DeviceId,
    window: WindowId,
    serial: u32,
}

/// Maps a window and clicks it twice (the first click only activates).
fn desk(limits: Option<((i32, i32), (i32, i32))>) -> Desk {
    let (mut fixture, id) = boot();
    let base = WmBase::bind(fixture.client(id), 7);
    let seat = bind_pointer(fixture.client(id));
    fixture.pump();
    let top = base.toplevel(fixture.client(id));
    if let Some((min, max)) = limits {
        top.set_limits(min, max);
    }
    top.commit_bare();
    fixture.pump();
    top.ack_latest();
    let buffer = casement_testclient::shm_buffer(
        fixture.client(id),
        casement_core::geometry::PhysicalSize { w: 400, h: 300 },
        casement_testclient::Pattern::Gradient,
    );
    top.show(&buffer);
    fixture.pump();
    settle(&mut fixture);
    let window = windows(&fixture)[0];
    let device = pointer(&mut fixture);
    click(&mut fixture, device, window);
    click(&mut fixture, device, window);
    let serial = seat.last_press().expect("a press was delivered");
    Desk {
        fixture,
        top,
        seat,
        device,
        window,
        serial,
    }
}

impl Desk {
    fn rect(&self) -> LogicalRect {
        rect_of(&self.fixture, self.window)
    }

    fn pointer_to(&mut self, at: LogicalPoint) {
        self.fixture.input().pointer_to(self.device, at);
    }

    fn button(&mut self, button: ButtonCode, edge: Edge) {
        self.fixture.input().button(self.device, button, edge);
    }
}

/// A grab ends when the button it began with is released, not when another one is.
#[test]
fn a_grab_ends_with_its_own_button() {
    let mut desk = desk(None);
    let start = desk.rect();
    desk.pointer_to(start.centre());
    desk.top.request_move(&desk.seat.seat, desk.serial);
    desk.fixture.pump();
    desk.button(BTN_LEFT, Edge::Press);
    let centre = start.centre();
    desk.pointer_to(LogicalPoint {
        x: centre.x + 50,
        y: centre.y,
    });
    assert_eq!(desk.rect().origin.x, start.origin.x + 50);
    // Another button comes and goes while the first is held: the grab goes on.
    desk.button(BTN_RIGHT, Edge::Press);
    desk.button(BTN_RIGHT, Edge::Release);
    desk.pointer_to(LogicalPoint {
        x: centre.x + 80,
        y: centre.y,
    });
    assert_eq!(
        desk.rect().origin.x,
        start.origin.x + 80,
        "a second button's release did not end the move"
    );
    // The first button's release ends it: the window stays where it was dropped.
    desk.button(BTN_LEFT, Edge::Release);
    desk.pointer_to(LogicalPoint {
        x: centre.x + 200,
        y: centre.y,
    });
    assert_eq!(desk.rect().origin.x, start.origin.x + 80);
}

/// A resize never makes the window smaller than its minimum or larger than its maximum, and
/// the edge opposite the one grabbed stays where it was.
#[test]
fn a_resize_respects_the_size_limits() {
    let mut desk = desk(Some(((300, 250), (500, 350))));
    let start = desk.rect();
    assert_eq!((start.size.w, start.size.h), (400, 300));
    let (right, bottom) = (
        start.origin.x + i32::try_from(start.size.w).unwrap_or(0),
        start.origin.y + i32::try_from(start.size.h).unwrap_or(0),
    );
    // Grab the top-left corner and drag it far into the window: the minimum holds the size and
    // the bottom-right corner stays put.
    desk.pointer_to(start.origin);
    desk.top
        .request_resize(&desk.seat.seat, desk.serial, ResizeEdge::TopLeft);
    desk.fixture.pump();
    desk.button(BTN_LEFT, Edge::Press);
    desk.pointer_to(LogicalPoint {
        x: start.origin.x + 250,
        y: start.origin.y + 200,
    });
    let shrunk = desk.rect();
    assert_eq!(
        (shrunk.size.w, shrunk.size.h),
        (300, 250),
        "the minimum holds"
    );
    assert_eq!(
        (
            shrunk.origin.x + i32::try_from(shrunk.size.w).unwrap_or(0),
            shrunk.origin.y + i32::try_from(shrunk.size.h).unwrap_or(0)
        ),
        (right, bottom),
        "the bottom-right corner did not move"
    );
    // Drag the same corner far out: the maximum holds, the corner opposite still stays.
    desk.pointer_to(LogicalPoint {
        x: start.origin.x - 400,
        y: start.origin.y - 400,
    });
    let grown = desk.rect();
    assert_eq!(
        (grown.size.w, grown.size.h),
        (500, 350),
        "the maximum holds"
    );
    assert_eq!(
        (
            grown.origin.x + i32::try_from(grown.size.w).unwrap_or(0),
            grown.origin.y + i32::try_from(grown.size.h).unwrap_or(0)
        ),
        (right, bottom),
        "the bottom-right corner did not move"
    );
    desk.button(BTN_LEFT, Edge::Release);
}

/// A resize tells the client the size it is dragged to (a configure per step), so it can draw
/// that size: found by dev/accept/map-resize.sh, where the frame grew and the client never knew.
#[test]
fn a_resize_configures_the_client_to_the_dragged_size() {
    let mut desk = desk(None);
    let start = desk.rect();
    let (right, bottom) = (
        start.origin.x + i32::try_from(start.size.w).unwrap_or(0),
        start.origin.y + i32::try_from(start.size.h).unwrap_or(0),
    );
    desk.pointer_to(LogicalPoint {
        x: right - 2,
        y: bottom - 2,
    });
    desk.top
        .request_resize(&desk.seat.seat, desk.serial, ResizeEdge::BottomRight);
    desk.fixture.pump();
    desk.button(BTN_LEFT, Edge::Press);
    desk.pointer_to(LogicalPoint {
        x: right + 98,
        y: bottom + 78,
    });
    desk.fixture.pump();
    let last = desk.top.configures().last().cloned().expect("a configure");
    assert_eq!(
        last.size,
        (500, 380),
        "the client was configured to the dragged size"
    );
    desk.button(BTN_LEFT, Edge::Release);
}

impl Desk {
    /// The state of the dragged window in the model.
    fn state(&self) -> WindowState {
        self.fixture.model().windows()[&self.window].state.clone()
    }

    /// The whole rectangle of the window's output.
    fn output_rect(&self) -> LogicalRect {
        let model = self.fixture.model();
        let output = model
            .output_of(&model.windows()[&self.window])
            .expect("the window is on an output");
        model
            .output_rects()
            .into_iter()
            .find(|(key, _)| *key == output)
            .map(|(_, rect)| rect)
            .expect("the output is plugged")
    }

    /// The rectangle `zone` takes of the window's output.
    fn zone(&self, zone: TileZone) -> LogicalRect {
        let model = self.fixture.model();
        let output = model
            .output_of(&model.windows()[&self.window])
            .expect("the window is on an output");
        let usable = model.layers().usable_area(&output, self.output_rect());
        zone_rect(zone, usable, LayoutKnobs::of(model.settings()).tile_margin)
    }

    /// A move of the window begins at its centre.
    fn begin_move(&mut self) {
        let centre = self.rect().centre();
        self.pointer_to(centre);
        self.top.request_move(&self.seat.seat, self.serial);
        self.fixture.pump();
        self.button(BTN_LEFT, Edge::Press);
    }
}

/// Dragging by the title to the left edge tiles the window to the left half on release, the
/// preview-then-tile steps of Sequoia (the preview itself is the render golden's).
#[test]
fn a_drag_to_the_left_edge_tiles_the_left_half() {
    let mut desk = desk(None);
    desk.begin_move();
    let y = desk.output_rect().centre().y;
    desk.pointer_to(LogicalPoint { x: 0, y });
    desk.button(BTN_LEFT, Edge::Release);
    assert!(
        matches!(
            desk.state(),
            WindowState::Tiled {
                zone: TileZone::Left,
                ..
            }
        ),
        "tiled to the left, not {:?}",
        desk.state()
    );
    let want = desk.zone(TileZone::Left);
    let got = desk.rect();
    assert_eq!((got.origin.x, got.size.w), (want.origin.x, want.size.w));
}

/// The top edge fills, a corner takes a quarter.
#[test]
fn the_top_edge_fills_and_a_corner_tiles_a_quarter() {
    type Spot = fn(LogicalRect) -> LogicalPoint;
    let cases: [(&str, Spot, Option<TileZone>); 2] = [
        (
            "top edge",
            |full: LogicalRect| LogicalPoint {
                x: full.centre().x,
                y: 0,
            },
            None,
        ),
        (
            "bottom right corner",
            |full: LogicalRect| LogicalPoint {
                x: full.origin.x + i32::try_from(full.size.w).unwrap_or(0) - 1,
                y: full.origin.y + i32::try_from(full.size.h).unwrap_or(0) - 1,
            },
            Some(TileZone::BottomRight),
        ),
    ];
    for (name, point, zone) in cases {
        let mut desk = desk(None);
        desk.begin_move();
        let at = point(desk.output_rect());
        desk.pointer_to(at);
        desk.button(BTN_LEFT, Edge::Release);
        match (zone, desk.state()) {
            (None, WindowState::Filled { .. }) => {}
            (Some(want), WindowState::Tiled { zone, .. }) if zone == want => {}
            (_, state) => panic!("{name}: the window is {state:?}"),
        }
    }
}

/// A drag that leaves the edge before the release, or never reaches one, leaves the window
/// floating where it was dropped.
#[test]
fn a_drag_off_the_edge_or_away_from_it_tiles_nothing() {
    let mut desk = desk(None);
    desk.begin_move();
    let centre = desk.output_rect().centre();
    desk.pointer_to(LogicalPoint { x: 0, y: centre.y });
    desk.pointer_to(LogicalPoint {
        x: 300,
        y: centre.y,
    });
    desk.button(BTN_LEFT, Edge::Release);
    assert!(
        matches!(desk.state(), WindowState::Floating(_)),
        "floating, not {:?}",
        desk.state()
    );
}

/// A window with the compositor's frame is dragged by its titlebar with a mouse: it follows
/// every relative step while it keeps drawing, and stays where it was let go (the trial login
/// of 2026-10-19, where the pointer did not move during a grab and every window stayed pinned
/// near the press).
#[test]
fn a_window_dragged_by_its_titlebar_stays_where_it_is_let_go() {
    for output in [
        "HEADLESS-1:1920x1080@60000:120:0,0",
        "HEADLESS-1:3840x2160@60000:240:0,0",
    ] {
        dragged_by_the_titlebar(output);
    }
}

fn dragged_by_the_titlebar(output: &str) {
    use casement_core::policy::ConnectionRole;
    use casement_testclient::misc::decoration::{DecorationManager, Mode};
    use casement_testkit::FixtureSpec;
    let mut fixture =
        Fixture::start(FixtureSpec::desktop().with_outputs(&[output])).expect("fixture");
    let id = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let base = WmBase::bind(fixture.client(id), 7);
    let manager = DecorationManager::bind(fixture.client(id)).expect("advertised");
    let top = base.toplevel(fixture.client(id));
    let decoration = manager.decorate(fixture.client(id), &top);
    top.commit_bare();
    fixture.pump();
    assert_eq!(decoration.newest(), Some(Mode::ServerSide));
    top.ack_latest();
    let buffer = casement_testclient::shm_buffer(
        fixture.client(id),
        casement_core::geometry::PhysicalSize { w: 400, h: 300 },
        casement_testclient::Pattern::Gradient,
    );
    top.show(&buffer);
    fixture.pump();
    settle(&mut fixture);
    let window = windows(&fixture)[0];
    let device = pointer(&mut fixture);
    let floating = |fixture: &Fixture| match fixture.model().windows()[&window].state {
        WindowState::Floating(rect) => Some(rect.origin),
        _ => None,
    };
    let start = rect_of(&fixture, window);
    let grip = LogicalPoint {
        x: start.centre().x,
        y: start.origin.y - 10,
    };
    fixture.input().pointer_to(device, grip);
    fixture.pump();
    fixture.input().button(device, BTN_LEFT, Edge::Press);
    fixture.pump();
    let mut acked = top.configures().len();
    let mut draw = |fixture: &mut Fixture| {
        if top.configures().len() > acked {
            acked = top.configures().len();
            top.ack_latest();
        }
        top.show(&buffer);
        fixture.pump();
    };
    let mut first = None;
    for step in 1..=10 {
        // A mouse moves by steps (relative motion), each from where the last one left it.
        fixture.input().pointer_by(
            device,
            SubPoint {
                x: SubPx(-12 * 120),
                y: SubPx(8 * 120),
            },
        );
        fixture.pump();
        draw(&mut fixture);
        let at = floating(&fixture).expect("floating while dragged");
        let first = *first.get_or_insert(LogicalPoint {
            x: at.x + 12,
            y: at.y - 8,
        });
        assert_eq!(
            at,
            LogicalPoint {
                x: first.x - 12 * step,
                y: first.y + 8 * step,
            },
            "{output} step {step} follows the pointer"
        );
    }
    let dropped = floating(&fixture);
    fixture.input().button(device, BTN_LEFT, Edge::Release);
    fixture.pump();
    draw(&mut fixture);
    settle(&mut fixture);
    draw(&mut fixture);
    assert_eq!(
        floating(&fixture),
        dropped,
        "{output}: the window stays where it was let go"
    );
    assert!(fixture.client(id).protocol_error().is_none());
}
