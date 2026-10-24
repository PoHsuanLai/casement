//! Minimise, as the real run checks it: Genie and Scale (`windows.minimise_effect`) both fly into
//! the dock and back out on restore, a flying window takes no clicks, and a frame caught
//! mid-flight at 1.5x and 2x is a golden (the strips of a genie meet without a seam).

use super::desk::{Desk, Dpi, OUTPUT, only_window};
use super::golden_matches;
use casement_core::geometry::LogicalPoint;
use casement_core::input::{ButtonCode, DeviceClass, Edge};
use casement_core::settings::MinimiseEffect;
use casement_core::{DeviceId, OutputName, WindowId};
use casement_testclient::Pattern;
use casement_testkit::{Fixture, FixtureSpec, FrameCount, ShotTarget, TestOrigin};
use smithay::reexports::wayland_server::Resource;
use std::time::Duration;
use wayland_client::Proxy;

const APP: &str = "org.example.minimise";

/// How often the flight is looked at.
const STEP: Duration = Duration::from_millis(25);

/// A desktop whose minimise effect is `effect`.
fn spec(effect: MinimiseEffect) -> FixtureSpec {
    let mut spec = FixtureSpec::desktop();
    spec.settings.windows.minimise_effect = effect;
    spec
}

/// How far `window` is along its minimise flight now, in thousandths; none when it is not on one.
fn flight_of(fixture: &Fixture, window: WindowId) -> Option<u32> {
    let scene = fixture.scene();
    let text = scene.0["frame"].as_str().expect("the frame as text");
    let line = text
        .lines()
        .find(|line| line.starts_with(&format!("w{} ", window.0)))?;
    line.split(" flight ").nth(1)?.trim().parse().ok()
}

/// The flight of `window` looked at every [`STEP`] for up to `bound`: its progress while it is
/// on one.
fn flown(fixture: &mut Fixture, window: WindowId, bound: Duration) -> Vec<u32> {
    let mut seen = Vec::new();
    let mut spent = Duration::ZERO;
    while spent < bound {
        if let Some(progress) = flight_of(fixture, window) {
            seen.push(progress);
        }
        fixture.advance(STEP);
        spent += STEP;
    }
    seen
}

/// cas-wave "Real-run checks: Minimise" (both effects play and reverse on restore): a window
/// minimised under either effect flies from its place toward the dock tile (progress rising from
/// nothing toward all of it) and is off the flight at the end; restored, it flies back out
/// (progress falling from the tile's end toward nothing) and lands where it was.
#[test]
fn genie_and_scale_both_play_and_reverse_on_restore() {
    for effect in [MinimiseEffect::Genie, MinimiseEffect::Scale] {
        let mut fixture = Fixture::start(spec(effect)).expect("fixture");
        assert_eq!(fixture.model().settings().windows.minimise_effect, effect);
        let mut desk = Desk::at(&mut fixture);
        let top = desk.window(APP, (600, 400), Pattern::Gradient, Dpi::One);
        let window = only_window(desk.fixture);
        desk.fixture.advance(Duration::from_secs(1));
        assert_eq!(flight_of(desk.fixture, window), None, "{effect:?}: at rest");

        top.set_minimized();
        desk.fixture.pump();
        let out = flown(desk.fixture, window, Duration::from_secs(2));
        assert!(out.len() >= 3, "{effect:?}: the minimise flies: {out:?}");
        assert!(out[0] < 300, "{effect:?}: it starts at the window: {out:?}");
        assert!(
            out.iter().copied().max() >= Some(700),
            "{effect:?}: it reaches toward the tile: {out:?}"
        );
        assert!(
            out.windows(2)
                .take_while(|pair| pair[0] < 900)
                .all(|pair| pair[1] >= pair[0]),
            "{effect:?}: the flight only goes on toward the tile: {out:?}"
        );
        assert_eq!(flight_of(desk.fixture, window), None, "{effect:?}: landed");

        desk.fixture.activate_window(window);
        let back = flown(desk.fixture, window, Duration::from_secs(2));
        assert!(back.len() >= 3, "{effect:?}: the restore flies: {back:?}");
        assert!(back[0] > 700, "{effect:?}: it starts at the tile: {back:?}");
        assert!(
            back.iter().copied().min() <= Some(300),
            "{effect:?}: it comes back out: {back:?}"
        );
        assert!(
            back.windows(2)
                .take_while(|pair| pair[0] > 100)
                .all(|pair| pair[1] <= pair[0]),
            "{effect:?}: the restore reverses the flight: {back:?}"
        );
        assert_eq!(flight_of(desk.fixture, window), None, "{effect:?}: back");
    }
}

/// A physical pointer.
fn mouse(fixture: &mut Fixture) -> DeviceId {
    fixture.input().plug(
        DeviceClass::Pointer,
        TestOrigin::Physical(DeviceClass::Pointer),
    )
}

/// The protocol id of the surface the pointer is on (both windows are one client's, so the
/// ids are unambiguous).
fn focus_id(fixture: &Fixture) -> Option<u32> {
    fixture
        .pointer_focus()
        .map(|surface| surface.id().protocol_id())
}

/// cas-wave "Real-run checks: Minimise" (a flying window takes no clicks): two windows lie one
/// over the other; the top one is minimised, and while it flies the pointer where it was lands
/// on the window below it, a press included.
#[test]
fn a_flying_window_takes_no_clicks() {
    let mut fixture = Fixture::start(spec(MinimiseEffect::Genie)).expect("fixture");
    let mut desk = Desk::at(&mut fixture);
    let below = desk.window(APP, (600, 400), Pattern::Checker { cell: 20 }, Dpi::One);
    let above = desk.window(APP, (600, 400), Pattern::Gradient, Dpi::One);
    let ids: Vec<WindowId> = desk.fixture.model().windows().keys().copied().collect();
    let (lower, upper) = match ids[..] {
        [lower, upper] => (lower, upper),
        ref other => panic!("two windows, got {other:?}"),
    };
    let at = LogicalPoint { x: 400, y: 300 };
    desk.fixture.place_window_by_hand(lower, at);
    desk.fixture.place_window_by_hand(upper, at);
    desk.fixture.advance(Duration::from_secs(1));
    let device = mouse(desk.fixture);
    let inside = LogicalPoint { x: 700, y: 500 };
    desk.fixture.input().pointer_to(device, inside);
    assert_eq!(
        focus_id(desk.fixture),
        Some(above.surface.id().protocol_id()),
        "before: the pointer is on the window above"
    );

    above.set_minimized();
    desk.fixture.pump();
    desk.fixture.advance(STEP * 2);
    assert!(
        flight_of(desk.fixture, upper).is_some(),
        "the window is flying"
    );
    desk.fixture
        .input()
        .pointer_to(device, LogicalPoint { x: 702, y: 502 });
    assert_eq!(
        focus_id(desk.fixture),
        Some(below.surface.id().protocol_id()),
        "while it flies the pointer is on what is under it"
    );
    desk.fixture
        .input()
        .button(device, ButtonCode(0x110), Edge::Press);
    desk.fixture
        .input()
        .button(device, ButtonCode(0x110), Edge::Release);
    assert_eq!(
        focus_id(desk.fixture),
        Some(below.surface.id().protocol_id()),
        "a press does not change that"
    );
}

/// The content's colour: an opaque magenta nothing else on the desktop has. A window fading into
/// the dock shows it washed with black, `(a, 0, a)`, and the grey titlebar `(g, g, g)`, so the
/// two can be told apart at any opacity.
const CONTENT: u32 = 0xffff_00ff;

/// How far from the output's corner the cursor (resting at the origin) reaches, in pixels.
const CURSOR_REACH: u32 = 64;

/// The left, top, right and bottom pixel of the pixels of `shot` that `is` accepts, off the
/// cursor's corner; none when there are none.
fn bounds(shot: &image::RgbaImage, is: impl Fn([u8; 4]) -> bool) -> Option<(u32, u32, u32, u32)> {
    shot.enumerate_pixels()
        .filter(|(x, y, pixel)| (*x >= CURSOR_REACH || *y >= CURSOR_REACH) && is(pixel.0))
        .fold(None, |found, (x, y, _)| match found {
            None => Some((x, y, x, y)),
            Some((l, t, r, b)) => Some((l.min(x), t.min(y), r.max(x), b.max(y))),
        })
}

/// What a frame of a window caught mid-flight must show, whatever the scale: the titlebar and
/// the content are both drawn, the content lies inside the frame's width and below its top (a
/// window whose frame shrinks while its content keeps its size would run past it), and under
/// Scale the content is as wide as the frame and on the screen.
fn assert_whole(shot: &image::RgbaImage, effect: MinimiseEffect, label: &str) {
    let bar = bounds(shot, |p| p[0] == p[1] && p[1] == p[2] && p[0] > 0)
        .unwrap_or_else(|| panic!("{label}: the titlebar is drawn"));
    let content = bounds(shot, |p| p[1] == 0 && p[0] > 0 && p[0] == p[2])
        .unwrap_or_else(|| panic!("{label}: the content is drawn"));
    let (bar_left, bar_top, bar_right, _) = bar;
    let (left, top, right, bottom) = content;
    assert!(
        left + 2 >= bar_left && right <= bar_right + 2,
        "{label}: the content {content:?} lies inside the frame's width {bar:?}"
    );
    assert!(
        top >= bar_top,
        "{label}: the content {content:?} is under the titlebar {bar:?}"
    );
    if matches!(effect, MinimiseEffect::Scale) {
        let (bar_width, width) = (bar_right - bar_left, right - left);
        assert!(
            bar_width.abs_diff(width) * 25 <= bar_width,
            "{label}: the content is as wide as the frame: {width} against {bar_width}"
        );
        assert!(
            bottom + 2 < shot.height(),
            "{label}: the content {content:?} is on the screen, not running off it"
        );
    }
}

/// One window flown part of the way into the dock under `effect` on an output of `output`
/// pixels at `dpi`, and the frame shot there.
fn mid_flight(effect: MinimiseEffect, output: &str, dpi: Dpi) -> image::RgbaImage {
    let spec = spec(effect).with_outputs(&[output]);
    let mut fixture = Fixture::start(spec).expect("fixture");
    let mut desk = Desk::at(&mut fixture);
    let top = desk.framed_window(APP, "Flying", (800, 500), Pattern::Solid(CONTENT), dpi);
    let window = only_window(desk.fixture);
    desk.fixture.advance(Duration::from_secs(1));
    top.set_minimized();
    desk.fixture.pump();
    let name = OutputName::parse(OUTPUT).expect("a name");
    for _ in 0..80 {
        if flight_of(desk.fixture, window).is_some_and(|progress| progress >= 400) {
            break;
        }
        desk.fixture.advance(STEP);
    }
    let progress = flight_of(desk.fixture, window).expect("still in flight");
    assert!(
        (400..=800).contains(&progress),
        "{effect:?} {output}: shot mid-flight, not at {progress}"
    );
    desk.fixture.frames(&name, FrameCount(1));
    desk.fixture.shot(ShotTarget::Output(name))
}

/// The 1.5x and 2x outputs the mid-flight checks run on.
const SCALED: [(&str, &str, Dpi); 2] = [
    (
        "1.5x",
        "HEADLESS-1:2880x1620@60000:180:0,0",
        Dpi::OneAndAHalf,
    ),
    ("2x", "HEADLESS-1:3840x2160@60000:240:0,0", Dpi::Two),
];

/// A frame of `effect` caught part of the way in, at 1.5x and 2x, shows the whole window
/// (`assert_whole`) and matches its golden `minimise-<effect>-<scale>-mid`.
fn whole_mid_flight(effect: MinimiseEffect) {
    for (scale, output, dpi) in SCALED {
        let shot = mid_flight(effect, output, dpi);
        let word = format!("{effect:?}").to_lowercase();
        assert_whole(&shot, effect, &format!("{word} {scale}"));
        golden_matches(&format!("minimise-{word}-{scale}-mid"), &shot);
    }
}

/// cas-wave "Real-run checks: Minimise" (no seams at 1.5x and 2x), for Scale: the content shrinks
/// with the frame.
#[test]
fn a_scale_flight_mid_way_is_whole_at_fractional_and_double_scale() {
    whole_mid_flight(MinimiseEffect::Scale);
}

/// The same for the Genie: its strips meet without a visible seam.
#[test]
fn a_genie_flight_mid_way_is_whole_at_fractional_and_double_scale() {
    whole_mid_flight(MinimiseEffect::Genie);
}

/// How many rows of the content in `shot` start (from the left) with a pixel between the
/// window's colour and the black behind it, out of how many rows have content at all. The
/// washed content is `(a, 0, a)` over black, so a blend is `(b, 0, b)` with `0 < b < a`.
fn soft_rows(shot: &image::RgbaImage) -> (u32, u32) {
    let content = |p: [u8; 4]| p[1] == 0 && p[0] > 0 && p[0] == p[2];
    (CURSOR_REACH..shot.height())
        .filter_map(|y| {
            let row: Vec<[u8; 4]> = (CURSOR_REACH..shot.width())
                .map(|x| shot.get_pixel(x, y).0)
                .collect();
            let first = row.iter().position(|p| content(*p))?;
            let inside = *row.get(first + 3).filter(|p| content(**p))?;
            Some(row[first][0] < inside[0])
        })
        .fold((0, 0), |(soft, all), is_soft| {
            (soft + u32::from(is_soft), all + 1)
        })
}

/// A genie's sides are smooth: caught mid-flight at 2x, the pixels beside the window's left
/// side are blends of its colour and the black behind (a share of the pixel covered), not
/// only the window's colour or nothing, which is what a side drawn in whole pixels steps through.
#[test]
fn the_sides_of_a_genie_are_anti_aliased() {
    let (_, output, dpi) = SCALED[1];
    let shot = mid_flight(MinimiseEffect::Genie, output, dpi);
    let (soft, all) = soft_rows(&shot);
    assert!(all > 100, "the content has {all} rows");
    assert!(
        soft * 4 >= all,
        "{soft} of {all} rows have a blended pixel on the left side"
    );
}
