//! A window with a frame above its content: fill, first placement and the keep-inside clamp
//! hold the outer frame (content plus the reserve) inside the usable area.

use crate::support;

use casement_core::WindowId;
use casement_core::geometry::LogicalRect;
use casement_layout::{ConfigureNeed, RustScene, SceneLayout, WindowPlacement};
use support::scene::{ANY_SIZE, knobs, one_desktop, rect, window};

const BAR: u32 = 28;

/// A name, the placement, the reserve, the content rectangle it must get.
type Case = (
    &'static str,
    WindowPlacement,
    u32,
    LogicalRect,
    Option<ConfigureNeed>,
);

#[test]
fn the_frame_stays_inside_the_usable_area() {
    let usable = rect(0, 24, 1920, 1056);
    let cases: Vec<Case> = vec![
        (
            "a bare window is unchanged",
            WindowPlacement::Floating(rect(100, 24, 400, 300)),
            0,
            rect(100, 24, 400, 300),
            Some(ConfigureNeed::Keep),
        ),
        (
            "the bar would sit under the menu bar: pushed down",
            WindowPlacement::Floating(rect(100, 24, 400, 300)),
            BAR,
            rect(100, 24 + 28, 400, 300),
            Some(ConfigureNeed::Keep),
        ),
        (
            "already clear of the top: kept",
            WindowPlacement::Floating(rect(100, 200, 400, 300)),
            BAR,
            rect(100, 200, 400, 300),
            Some(ConfigureNeed::Keep),
        ),
        (
            "off the top",
            WindowPlacement::Floating(rect(100, -500, 400, 300)),
            BAR,
            rect(100, 52, 400, 300),
            None,
        ),
        (
            "below: the content's foot stays inside",
            WindowPlacement::Floating(rect(100, 5000, 400, 300)),
            BAR,
            rect(100, 24 + 1056 - 300, 400, 300),
            None,
        ),
        (
            "a fill leaves the bar's rows",
            WindowPlacement::Filled,
            BAR,
            rect(0, 52, 1920, 1028),
            None,
        ),
        (
            "a fill without a bar",
            WindowPlacement::Filled,
            0,
            rect(0, 24, 1920, 1056),
            None,
        ),
        (
            "too tall for the area: held to the rest",
            WindowPlacement::Floating(rect(0, 24, 400, 5000)),
            BAR,
            rect(0, 52, 400, 1028),
            Some(ConfigureNeed::Resize(support::scene::size(400, 1028))),
        ),
        (
            "put by hand keeps its place, its size is fitted",
            WindowPlacement::Placed(rect(10, 0, 400, 5000)),
            BAR,
            rect(10, 0, 400, 1028),
            None,
        ),
    ];
    for (name, placement, reserve, want, need) in cases {
        let mut framed = window(1, placement, ANY_SIZE);
        framed.reserve_top = reserve;
        let spec = one_desktop(usable, vec![framed]);
        let arrangement = RustScene.arrange(&spec, &knobs());
        let got = arrangement
            .windows
            .iter()
            .find(|t| t.window == WindowId(1))
            .expect("a target");
        assert_eq!(got.rect, want, "{name}");
        if let Some(need) = need {
            assert_eq!(got.size, need, "{name} configure");
        }
    }
}

#[test]
fn a_new_window_starts_below_the_bar() {
    let usable = rect(0, 24, 800, 600);
    let mut framed = window(
        1,
        WindowPlacement::New(support::scene::size(800, 600)),
        ANY_SIZE,
    );
    framed.reserve_top = BAR;
    let arrangement = RustScene.arrange(&one_desktop(usable, vec![framed]), &knobs());
    let got = arrangement.windows[0].rect;
    assert_eq!(
        (got.origin.y, got.size.h),
        (52, 572),
        "the content takes what the bar leaves"
    );
}
