//! The scenes, by name: what each one maps and where the pointer rests.

use crate::scene::desk::{Desk, Dpi};
use casement_core::geometry::{LogicalPoint, PhysicalSize};
use casement_testclient::misc::background_effect::BlurAsk;
use casement_testclient::surface::layer::{
    Anchor, KeyboardInteractivity, Layer, LayerAsk, LayerSurface,
};
use casement_testclient::surface::lock::Locker;
use casement_testclient::surface::popup::PopupSpec;
use casement_testclient::{Pattern, shm_buffer};
use casement_testkit::Fixture;

/// The app all the windows of a scene belong to.
const APP: &str = "org.example.scene";

/// A translucent white wash: ARGB8888, premultiplied, a quarter opaque.
const WASH: u32 = 0x4040_4040;

/// Builds scene `name` on `fixture` and shoots its output.
pub fn scene(fixture: &mut Fixture, name: &str) -> image::RgbaImage {
    let mut desk = Desk::at(fixture);
    match name {
        "empty-desktop-1x" => desk.pointer_at(LogicalPoint { x: 960, y: 540 }),
        "one-window-1x" => one_window(&mut desk, Dpi::One),
        "one-window-1.5x" => one_window(&mut desk, Dpi::OneAndAHalf),
        "one-window-2x" => one_window(&mut desk, Dpi::Two),
        "cascaded-windows-1x" => cascaded(&mut desk),
        "popup-over-window-1x" => popup_over_window(&mut desk),
        "bar-and-window-1x" => bar_and_window(&mut desk),
        "lock-surface-1x" => locked(&mut desk, Survivor::Alive),
        "lock-cover-dead-locker-1x" => locked(&mut desk, Survivor::Dead),
        "named-cursor-1.5x" => named_cursor(&mut desk),
        "blur-v0-1x" => blur(&mut desk),
        other => panic!("no scene named {other}"),
    }
    let shot = desk.shoot();
    if name == "lock-cover-dead-locker-1x" {
        assert_nothing_but_the_cursor(&shot);
    }
    shot
}

/// How far from the output's corner the cursor (resting at the origin) reaches.
const CURSOR_REACH: u32 = 32;

/// A dead locker's cover shows nothing of the session: every pixel off the cursor is black.
fn assert_nothing_but_the_cursor(shot: &image::RgbaImage) {
    let leaked = shot
        .enumerate_pixels()
        .filter(|(x, y, _)| *x >= CURSOR_REACH || *y >= CURSOR_REACH)
        .filter(|(_, _, pixel)| pixel.0 != [0, 0, 0, 255])
        .count();
    assert_eq!(
        leaked, 0,
        "the cover left {leaked} pixels of the session showing"
    );
}

/// One 400x300 gradient window, drawn for `dpi`.
fn one_window(desk: &mut Desk<'_>, dpi: Dpi) {
    let top = desk.window(APP, (400, 300), Pattern::Gradient, dpi);
    desk.kept.keep(top);
}

/// Three windows of one app: the first centred, the rest cascaded.
fn cascaded(desk: &mut Desk<'_>) {
    let looks = [
        ((480, 320), Pattern::Checker { cell: 20 }),
        ((420, 300), Pattern::Gradient),
        ((360, 240), Pattern::CoordinateEncoding),
    ];
    for (size, pattern) in looks {
        let top = desk.window(APP, size, pattern, Dpi::One);
        desk.kept.keep(top);
    }
}

/// A menu hanging from a window.
fn popup_over_window(desk: &mut Desk<'_>) {
    let top = desk.window(APP, (600, 400), Pattern::Checker { cell: 25 }, Dpi::One);
    desk.popup(
        &top,
        PopupSpec::menu_at((40, 60)),
        Pattern::Gradient,
        Dpi::One,
    );
    desk.kept.keep(top);
}

/// A 28 px bar on the top edge that reserves its zone, and a window below it.
fn bar_and_window(desk: &mut Desk<'_>) {
    let bar = LayerAsk {
        level: Layer::Top,
        namespace: "bar",
        anchor: Anchor::Top | Anchor::Left | Anchor::Right,
        exclusive_zone: 28,
        keyboard: KeyboardInteractivity::None,
        size: (0, 28),
    };
    let layer = LayerSurface::create(desk.fixture.client(desk.id), &bar).expect("a layer surface");
    desk.fixture.pump();
    assert!(layer.ack(desk.fixture.client(desk.id)));
    let size = PhysicalSize { w: 1920, h: 28 };
    let buffer = shm_buffer(
        desk.fixture.client(desk.id),
        size,
        Pattern::Solid(0xff30_3a50),
    );
    layer.show(desk.fixture.client(desk.id), &buffer);
    desk.fixture.pump();
    let top = desk.window(APP, (800, 500), Pattern::Gradient, Dpi::One);
    desk.kept.keep((layer, buffer, top));
}

/// Whether the locker is still there when the scene is shot.
enum Survivor {
    Alive,
    Dead,
}

/// A window, then a session lock whose surface covers the output; a dead locker leaves the
/// compositor's own cover.
fn locked(desk: &mut Desk<'_>, locker: Survivor) {
    let top = desk.window(APP, (400, 300), Pattern::Gradient, Dpi::One);
    assert_eq!(
        desk.fixture.model().windows().len(),
        1,
        "a window is behind the lock"
    );
    let owner = desk.another();
    let client = desk.fixture.client(owner);
    let lock = Locker::request(client).expect("a lock");
    desk.fixture.pump();
    let cover = lock
        .cover(desk.fixture.client(owner))
        .expect("a lock surface");
    desk.fixture.pump();
    assert!(cover.ack(desk.fixture.client(owner)));
    let size = PhysicalSize { w: 1920, h: 1080 };
    let buffer = shm_buffer(
        desk.fixture.client(owner),
        size,
        Pattern::CoordinateEncoding,
    );
    cover.show(desk.fixture.client(owner), &buffer);
    desk.fixture.pump();
    assert!(lock.is_locked(), "the session is locked");
    match locker {
        Survivor::Alive => desk.kept.keep((lock, cover, buffer)),
        Survivor::Dead => desk.fixture.disconnect(owner),
    }
    desk.kept.keep(top);
}

/// The default arrow over a window at 1.5x.
fn named_cursor(desk: &mut Desk<'_>) {
    one_window(desk, Dpi::OneAndAHalf);
    desk.pointer_at(LogicalPoint { x: 900, y: 500 });
}

/// A translucent window with blur behind it, over a busy one.
fn blur(desk: &mut Desk<'_>) {
    let behind = desk.window(APP, (800, 500), Pattern::Checker { cell: 8 }, Dpi::One);
    let mut ask = BlurAsk::start(desk.fixture.client(desk.id).globals()).expect("the effect");
    let surface = ask.surface().clone();
    let whole = casement_core::geometry::LogicalRect {
        origin: LogicalPoint { x: 0, y: 0 },
        size: casement_core::geometry::LogicalSize { w: 420, h: 300 },
    };
    ask.set_blur(&[whole]).expect("a blur region");
    let front = desk.window_on(surface, APP, (420, 300), Pattern::Solid(WASH), Dpi::One);
    desk.kept.keep((behind, ask, front));
}
