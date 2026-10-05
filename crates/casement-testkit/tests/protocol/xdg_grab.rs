//! Interactive moves and resizes (`xdg_toplevel.move`, `xdg_toplevel.resize`): what ends a
//! grab, and what a resize may make of a window with size limits (P0-stubs).
//!
//! xdg-shell: the request answers a button press by its serial, and the grab lasts while that
//! button is held. A resize "respects" the window's declared minimum and maximum size: the
//! side the person did not grab does not move.

use crate::xdg_shell::{boot, click, pointer, rect_of, settle, windows};
use casement_core::DeviceId;
use casement_core::WindowId;
use casement_core::geometry::{LogicalPoint, LogicalRect};
use casement_core::input::{ButtonCode, Edge};
use casement_testclient::surface::toplevel::{
    BoundPointer, ResizeEdge, Toplevel, WmBase, bind_pointer,
};
use casement_testkit::{ClientId, Fixture};

const BTN_LEFT: ButtonCode = ButtonCode(0x110);
const BTN_RIGHT: ButtonCode = ButtonCode(0x111);

/// A mapped window of `size` with the declared `limits` (min, max), the pointer and the press
/// serial of a delivered left click.
struct Desk {
    fixture: Fixture,
    id: ClientId,
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
        id,
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

    /// Puts the pointer at the window's centre plus `by`.
    fn pointer_from_centre(&mut self, by: (i32, i32)) {
        let at = self.rect().centre();
        self.pointer_to(LogicalPoint {
            x: at.x + by.0,
            y: at.y + by.1,
        });
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
    let _ = desk.id;
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
    let _ = desk.pointer_from_centre((0, 0));
}
