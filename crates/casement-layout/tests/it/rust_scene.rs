//! RustScene's P0 rules: floating windows, centre-then-cascade, Space origins (the tables and
//! the properties P0-F1-layout makes pass).

use crate::support;

use casement_core::WindowId;
use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize};
use casement_core::window::{MaxSize, SizeLimits};
use casement_layout::{
    Arrangement, ConfigureNeed, LayoutReport, RustScene, SceneLayout, SpaceTarget, WindowPlacement,
    WindowTarget,
};
use support::scene::{ANY_SIZE, knobs, one_desktop, output_key, rect, size, space_id, window};

/// A window, its expected rectangle and configure need.
type Placed = (u64, LogicalRect, ConfigureNeed);

/// A name, the windows' placements, what each must get.
type CascadeCase = (&'static str, Vec<(u64, WindowPlacement)>, Vec<Placed>);

/// A name, the floating rectangle, its limits, the expected rectangle and configure need.
type FloatingCase = (
    &'static str,
    LogicalRect,
    SizeLimits,
    LogicalRect,
    ConfigureNeed,
);

fn target(arrangement: &Arrangement, n: u64) -> &WindowTarget {
    arrangement
        .windows
        .iter()
        .find(|t| t.window == WindowId(n))
        .expect("every window has a target")
}

#[test]
fn new_windows_centre_then_cascade() {
    let usable = rect(0, 24, 1920, 1056);
    let cases: Vec<CascadeCase> = vec![
        (
            "one new window is centred",
            vec![(1, WindowPlacement::New(size(800, 600)))],
            vec![(1, rect(560, 252, 800, 600), ConfigureNeed::Keep)],
        ),
        (
            "the second cascades one step",
            vec![
                (1, WindowPlacement::New(size(800, 600))),
                (2, WindowPlacement::New(size(800, 600))),
            ],
            vec![
                (1, rect(560, 252, 800, 600), ConfigureNeed::Keep),
                (2, rect(588, 280, 800, 600), ConfigureNeed::Keep),
            ],
        ),
        (
            "floating windows do not count toward the cascade",
            vec![
                (1, WindowPlacement::Floating(rect(10, 40, 300, 200))),
                (2, WindowPlacement::New(size(800, 600))),
            ],
            vec![
                (1, rect(10, 40, 300, 200), ConfigureNeed::Keep),
                (2, rect(560, 252, 800, 600), ConfigureNeed::Keep),
            ],
        ),
        (
            "a new window larger than the area is shrunk to it",
            vec![(1, WindowPlacement::New(size(2400, 1400)))],
            vec![(1, usable, ConfigureNeed::Resize(size(1920, 1056)))],
        ),
    ];
    for (name, windows, want) in cases {
        let spec = one_desktop(
            usable,
            windows
                .into_iter()
                .map(|(n, p)| window(n, p, ANY_SIZE))
                .collect(),
        );
        let got = RustScene.arrange(&spec, &knobs());
        for (n, rect, need) in want {
            let t = target(&got, n);
            assert_eq!((t.rect, t.size), (rect, need), "{name}: window {n}");
        }
    }
}

/// A window put by hand (a test's placement) keeps its position outside the usable area; the
/// same rectangle as a floating window (where the person dragged it) is pushed inside.
#[test]
fn a_hand_placed_window_may_leave_the_area_a_floating_one_may_not() {
    let usable = rect(0, 24, 1920, 1056);
    let at = rect(64, -5, 300, 300);
    let cases = [
        (
            "floating: pushed inside",
            WindowPlacement::Floating(at),
            rect(64, 24, 300, 300),
        ),
        ("placed by hand: kept", WindowPlacement::Placed(at), at),
    ];
    for (name, placement, want) in cases {
        let spec = one_desktop(usable, vec![window(1, placement, ANY_SIZE)]);
        let got = RustScene.arrange(&spec, &knobs());
        let t = target(&got, 1);
        assert_eq!((t.rect, t.size), (want, ConfigureNeed::Keep), "{name}");
    }
}

#[test]
fn a_filled_window_takes_the_usable_area_within_its_limits() {
    let usable = rect(0, 24, 1920, 1056);
    let max = SizeLimits {
        min: LogicalSize { w: 0, h: 0 },
        max: MaxSize::Bounded(size(1000, 700)),
    };
    let cases = [
        ("no limits: the whole area", ANY_SIZE, size(1920, 1056)),
        ("a maximum is respected", max, size(1000, 700)),
    ];
    for (name, limits, want) in cases {
        let spec = one_desktop(usable, vec![window(1, WindowPlacement::Filled, limits)]);
        let got = RustScene.arrange(&spec, &knobs());
        let t = target(&got, 1);
        assert_eq!(
            (t.rect, t.size),
            (
                LogicalRect::new(usable.origin, want),
                ConfigureNeed::Resize(want)
            ),
            "{name}"
        );
    }
}

#[test]
fn a_full_screen_window_covers_the_whole_output_whatever_its_limits() {
    let full = rect(0, 0, 1920, 1080);
    let usable = rect(0, 24, 1920, 1056);
    let max = SizeLimits {
        min: LogicalSize { w: 0, h: 0 },
        max: MaxSize::Bounded(size(1000, 700)),
    };
    for (name, limits) in [("no limits", ANY_SIZE), ("a maximum", max)] {
        let mut spec = one_desktop(usable, vec![window(1, WindowPlacement::FullScreen, limits)]);
        spec.outputs[0].full = full;
        let got = RustScene.arrange(&spec, &knobs());
        let t = target(&got, 1);
        assert_eq!(
            (t.rect, t.size),
            (full, ConfigureNeed::Resize(full.size)),
            "{name}"
        );
    }
}

#[test]
fn floating_windows_stay_or_come_back_within_reach() {
    let usable = rect(0, 24, 1920, 1056);
    let min = |w, h| SizeLimits {
        min: LogicalSize { w, h },
        max: MaxSize::Unbounded,
    };
    let cases: Vec<FloatingCase> = vec![
        (
            "inside stays",
            rect(100, 100, 400, 300),
            ANY_SIZE,
            rect(100, 100, 400, 300),
            ConfigureNeed::Keep,
        ),
        (
            "partly off the right edge stays",
            rect(1800, 100, 400, 300),
            ANY_SIZE,
            rect(1800, 100, 400, 300),
            ConfigureNeed::Keep,
        ),
        (
            "past the right edge comes back until its grip shows",
            rect(2500, 100, 400, 300),
            ANY_SIZE,
            rect(1920 - 112, 100, 400, 300),
            ConfigureNeed::Keep,
        ),
        (
            "above the bar moves down",
            rect(100, 0, 400, 300),
            ANY_SIZE,
            rect(100, 24, 400, 300),
            ConfigureNeed::Keep,
        ),
        (
            "too tall shrinks",
            rect(100, 24, 400, 2000),
            ANY_SIZE,
            rect(100, 24, 400, 1056),
            ConfigureNeed::Resize(size(400, 1056)),
        ),
        (
            "a minimum larger than the area yields to it and is reported",
            rect(0, 24, 400, 2000),
            min(400, 1200),
            rect(0, 24, 400, 1056),
            ConfigureNeed::Resize(size(400, 1056)),
        ),
    ];
    for (name, at, limits, want_rect, want_need) in cases {
        let spec = one_desktop(
            usable,
            vec![window(1, WindowPlacement::Floating(at), limits)],
        );
        let got = RustScene.arrange(&spec, &knobs());
        let t = target(&got, 1);
        assert_eq!((t.rect, t.size), (want_rect, want_need), "{name}");
        let unmet = limits.min.w > usable.size.w || limits.min.h > usable.size.h;
        let want_reports = if unmet {
            vec![LayoutReport::MinUnmet {
                window: WindowId(1),
                min: limits.min,
                got: t.rect.size,
            }]
        } else {
            Vec::new()
        };
        assert_eq!(got.reports, want_reports, "{name}");
    }
}

#[test]
fn the_shown_space_sits_at_the_usable_origin() {
    let usable = rect(0, 24, 1920, 1056);
    let got = RustScene.arrange(&one_desktop(usable, Vec::new()), &knobs());
    assert_eq!(
        got.spaces,
        vec![SpaceTarget {
            space: space_id("s-1"),
            output: output_key("HEADLESS-1"),
            origin: LogicalPoint { x: 0, y: 24 },
        }]
    );
    assert_eq!(
        (got.overview, got.space_thumbs, got.reports),
        (Vec::new(), Vec::new(), Vec::new())
    );
}

/// Three Spaces on one output, the second shown, a window in each: the strip is one usable
/// width apart, every window arranged as if its Space were shown.
#[test]
fn spaces_sit_one_usable_width_apart_around_the_shown_one() {
    use casement_layout::{SceneMode, SpaceArrangement, SpaceSpec};
    let usable = rect(40, 24, 1000, 700);
    let mut spec = one_desktop(usable, Vec::new());
    let output = &mut spec.outputs[0];
    output.spaces = ["s-1", "s-2", "s-3"]
        .iter()
        .zip(1u64..)
        .map(|(id, n)| SpaceSpec {
            space: space_id(id),
            kind: SpaceArrangement::Desktop,
            windows: vec![window(
                n,
                WindowPlacement::Floating(rect(100, 100, 300, 200)),
                ANY_SIZE,
            )],
        })
        .collect();
    output.mode = SceneMode::Desktop {
        shown: space_id("s-2"),
    };
    let got = RustScene.arrange(&spec, &knobs());
    let origins: Vec<_> = got
        .spaces
        .iter()
        .map(|s| (s.space.as_str(), s.origin))
        .collect();
    let at = |x, y| LogicalPoint { x, y };
    assert_eq!(
        origins,
        vec![
            ("s-1", at(-960, 24)),
            ("s-2", at(40, 24)),
            ("s-3", at(1040, 24))
        ]
    );
    let rects: Vec<_> = got.windows.iter().map(|t| t.rect).collect();
    assert_eq!(rects, vec![rect(100, 100, 300, 200); 3]);
}

/// Windows with no target at P0: minimised ones, and anything P3-Z arranges.
#[test]
fn minimised_windows_have_no_target() {
    let usable = rect(0, 24, 800, 600);
    let spec = one_desktop(
        usable,
        vec![
            window(1, WindowPlacement::Minimised, ANY_SIZE),
            window(
                2,
                WindowPlacement::Floating(rect(10, 40, 100, 100)),
                ANY_SIZE,
            ),
        ],
    );
    let got = RustScene.arrange(&spec, &knobs());
    let ids: Vec<_> = got.windows.iter().map(|t| t.window).collect();
    assert_eq!(ids, vec![WindowId(2)]);
}

#[test]
fn a_sheet_hangs_from_its_window_and_follows_it() {
    let usable = rect(0, 24, 1920, 1056);
    let sheet = WindowPlacement::Sheet {
        parent: WindowId(1),
        size: size(400, 200),
    };
    let rows = [
        (
            "centred",
            rect(560, 252, 800, 600),
            rect(760, 252, 400, 200),
        ),
        (
            "moved with it",
            rect(100, 100, 800, 600),
            rect(300, 100, 400, 200),
        ),
    ];
    for (name, parent, want) in rows {
        let spec = one_desktop(
            usable,
            vec![
                window(1, WindowPlacement::Floating(parent), ANY_SIZE),
                window(2, sheet.clone(), ANY_SIZE),
            ],
        );
        let got = RustScene.arrange(&spec, &knobs());
        assert_eq!(target(&got, 2).rect, want, "{name}");
        assert_eq!(target(&got, 2).size, ConfigureNeed::Keep, "{name}");
    }
    let orphan = one_desktop(usable, vec![window(2, sheet, ANY_SIZE)]);
    let got = RustScene.arrange(&orphan, &knobs());
    assert_eq!(
        target(&got, 2).rect,
        rect(760, 452, 400, 200),
        "with no window it is a new one"
    );
}

#[test]
fn zones_take_halves_and_quarters_inside_the_margin() {
    use casement_layout::zone_rect;
    use casement_protocols::vocab::TileZone as Z;
    let usable = rect(0, 24, 1920, 1056);
    // name, zone, area, margin, want
    let cases = [
        ("left", Z::Left, usable, 8, rect(8, 32, 948, 1040)),
        ("right", Z::Right, usable, 8, rect(964, 32, 948, 1040)),
        ("top", Z::Top, usable, 8, rect(8, 32, 1904, 516)),
        ("bottom", Z::Bottom, usable, 8, rect(8, 556, 1904, 516)),
        ("top left", Z::TopLeft, usable, 8, rect(8, 32, 948, 516)),
        ("top right", Z::TopRight, usable, 8, rect(964, 32, 948, 516)),
        (
            "bottom left",
            Z::BottomLeft,
            usable,
            8,
            rect(8, 556, 948, 516),
        ),
        (
            "bottom right",
            Z::BottomRight,
            usable,
            8,
            rect(964, 556, 948, 516),
        ),
        (
            "no margin, left",
            Z::Left,
            usable,
            0,
            rect(0, 24, 960, 1056),
        ),
        (
            "no margin, bottom right",
            Z::BottomRight,
            usable,
            0,
            rect(960, 552, 960, 528),
        ),
        (
            "odd width: the extra pixel is the second half's",
            Z::Right,
            rect(0, 0, 101, 100),
            0,
            rect(50, 0, 51, 100),
        ),
        (
            "odd width with a margin",
            Z::Right,
            rect(0, 0, 101, 100),
            8,
            rect(54, 8, 39, 84),
        ),
        (
            "a margin too big for the area is held to a quarter",
            Z::Right,
            rect(0, 0, 20, 20),
            8,
            rect(12, 5, 3, 10),
        ),
        (
            "an area off the origin (a second output)",
            Z::Left,
            rect(1920, 0, 1280, 720),
            8,
            rect(1928, 8, 628, 704),
        ),
    ];
    for (name, zone, area, margin, want) in cases {
        assert_eq!(zone_rect(zone, area, margin), want, "{name}");
    }
}

#[test]
fn a_tiled_window_takes_its_zone_below_its_frame_within_its_limits() {
    use casement_protocols::vocab::TileZone;
    let usable = rect(0, 24, 1920, 1056);
    let wide = SizeLimits {
        min: LogicalSize { w: 1000, h: 0 },
        max: MaxSize::Unbounded,
    };
    // name, limits, reserve, the rectangle, the report
    let cases = [
        ("no frame", ANY_SIZE, 0, rect(8, 32, 948, 1040), false),
        (
            "a titlebar comes off the top",
            ANY_SIZE,
            28,
            rect(8, 60, 948, 1012),
            false,
        ),
        (
            "a minimum the half cannot hold",
            wide,
            0,
            rect(8, 32, 948, 1040),
            true,
        ),
    ];
    for (name, limits, reserve, want, reported) in cases {
        let mut held = window(1, WindowPlacement::Tiled(TileZone::Left), limits);
        held.reserve_top = reserve;
        let got = RustScene.arrange(&one_desktop(usable, vec![held]), &knobs());
        let t = target(&got, 1);
        assert_eq!(
            (t.rect, t.size),
            (want, ConfigureNeed::Resize(want.size)),
            "{name}"
        );
        assert_eq!(!got.reports.is_empty(), reported, "{name}");
    }
}
