//! A Space that is not shown is still arranged, as a desktop with no motion: its windows have
//! the places they would have if it were shown. Mission Control's Space pictures read them.

use crate::support;

use casement_core::WindowId;
use casement_core::geometry::LogicalRect;
use casement_layout::{
    Arrangement, LayoutKnobs, OutputSpec, RustScene, SceneLayout, SceneMode, SceneSpec,
    SpaceArrangement, SpaceSpec, WindowPlacement,
};
use ds_core::geometry::scale::Scale;
use support::scene::{ANY_SIZE, knobs, output_key, rect, size, space_id, window};

/// One output with two desktops: `s-1` with window 1, `s-2` with the windows 2, 3 and 4; the
/// Space `shown` is the one the output shows.
fn two_spaces(shown: &str) -> SceneSpec {
    let usable = rect(0, 24, 1920, 1056);
    let desktop = |id: &str, windows| SpaceSpec {
        space: space_id(id),
        kind: SpaceArrangement::Desktop,
        windows,
    };
    SceneSpec {
        outputs: vec![OutputSpec {
            output: output_key("HEADLESS-1"),
            full: usable,
            covered: usable,
            usable,
            scale: Scale::ONE,
            mode: SceneMode::Desktop {
                shown: space_id(shown),
            },
            spaces: vec![
                desktop(
                    "s-1",
                    vec![window(
                        1,
                        WindowPlacement::Floating(rect(10, 40, 300, 200)),
                        ANY_SIZE,
                    )],
                ),
                desktop(
                    "s-2",
                    vec![
                        window(
                            2,
                            WindowPlacement::Floating(rect(100, 100, 400, 300)),
                            ANY_SIZE,
                        ),
                        window(3, WindowPlacement::New(size(800, 600)), ANY_SIZE),
                        window(4, WindowPlacement::Filled, ANY_SIZE),
                    ],
                ),
            ],
        }],
    }
}

fn rect_of(arrangement: &Arrangement, n: u64) -> LogicalRect {
    arrangement
        .windows
        .iter()
        .find(|target| target.window == WindowId(n))
        .unwrap_or_else(|| panic!("window {n} has no target"))
        .rect
}

fn arranged(shown: &str, knobs: &LayoutKnobs) -> Arrangement {
    RustScene.arrange(&two_spaces(shown), knobs)
}

#[test]
fn a_hidden_spaces_windows_are_placed_as_if_it_were_shown() {
    let knobs = knobs();
    let hidden = arranged("s-1", &knobs);
    let shown = arranged("s-2", &knobs);
    // name, window, where it sits (usable area from y 24)
    let cases = [
        ("floating", 2, rect(100, 100, 400, 300)),
        ("new, centred", 3, rect(560, 252, 800, 600)),
        ("filled", 4, rect(0, 24, 1920, 1056)),
    ];
    for (name, window, want) in cases {
        assert_eq!(rect_of(&hidden, window), want, "{name}, hidden");
        assert_eq!(
            rect_of(&hidden, window),
            rect_of(&shown, window),
            "{name}: the same shown or not"
        );
    }
}

#[test]
fn the_shown_space_keeps_its_places_whichever_space_is_hidden() {
    let knobs = knobs();
    assert_eq!(rect_of(&arranged("s-1", &knobs), 1), rect(10, 40, 300, 200));
    assert_eq!(
        rect_of(&arranged("s-2", &knobs), 1),
        rect(10, 40, 300, 200),
        "window 1 is placed while its Space is hidden too"
    );
}

#[test]
fn every_window_of_every_space_has_one_target_on_its_output() {
    let arrangement = arranged("s-1", &knobs());
    let mut seen: Vec<u64> = arrangement.windows.iter().map(|t| t.window.0).collect();
    seen.sort_unstable();
    assert_eq!(seen, vec![1, 2, 3, 4]);
    assert!(
        arrangement
            .windows
            .iter()
            .all(|target| target.output == output_key("HEADLESS-1"))
    );
}
