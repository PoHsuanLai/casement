//! What each cause and level does to windows and Space strips: continuity at a retarget,
//! reduced motion settling at once, arrivals, departures and the minimise travel, the stack
//! order of a frame, and how a window is looked at.

use crate::support;

use casement_core::geometry::{LogicalPoint, LogicalSize, SubPx};
use casement_core::{Cause, MinimiseTarget, SwipeRelease, WindowId};
use casement_layout::{Arrangement, SpaceTarget};
use casement_motion::{
    CommittedSizes, MotionBook, Settled, WindowLooks, retarget, retarget_driven, sample,
};
use casement_protocols::vocab::{Emphasis, FrameMode};
use ds_core::vocab::Fraction;
use ds_motion::velocity::Velocity;
use ds_style::appearance::motion::MotionLevel;
use porter_core::SpaceId;
use std::collections::BTreeSet;
use support::mc_entry::{arrangements, journeys};
use support::{arranged, at, committed, drawn, look, output, placed, rect};

const STANDARD: MotionLevel = MotionLevel::Standard;
const REDUCED: MotionLevel = MotionLevel::Reduced;

/// A book that knows `arrangement` at rest at 0 ms.
fn known(arrangement: &Arrangement, level: MotionLevel) -> MotionBook {
    retarget(
        MotionBook::empty(),
        arrangement,
        arrangement,
        &Cause::Configure,
        level,
        at(0),
    )
}

#[test]
fn a_retarget_mid_flight_has_no_sub_pixel_jump_on_any_channel() {
    let (desktop, overview) = arrangements();
    let causes = [Cause::Overview, Cause::Keyboard, Cause::Drop];
    for (cause, back) in causes.iter().zip([true, false, true]) {
        let (from, to) = (&desktop, &overview);
        let first = retarget(known(from, STANDARD), from, to, cause, STANDARD, at(0));
        // aimed back where it came from, or at a third place
        let third = Arrangement {
            windows: to
                .windows
                .iter()
                .map(|t| {
                    let mut t = t.clone();
                    t.rect.origin = LogicalPoint {
                        x: t.rect.origin.x + 300,
                        y: t.rect.origin.y - 20,
                    };
                    t
                })
                .collect(),
            ..Arrangement::default()
        };
        let next = if back { from } else { &third };
        for ms in [20u64, 90, 200, 310] {
            let second = retarget(first.clone(), to, next, cause, STANDARD, at(ms));
            for (n, ..) in journeys() {
                assert_eq!(
                    drawn(&first, n, at(ms)),
                    drawn(&second, n, at(ms)),
                    "window {n} jumped at {ms} ms ({cause:?})"
                );
            }
        }
    }
}

#[test]
fn chained_retargets_end_exactly_at_the_last_target() {
    let a = arranged(&[(1, "HEADLESS-1", rect(0, 0, 400, 300))]);
    let b = arranged(&[(1, "HEADLESS-1", rect(900, 40, 500, 350))]);
    let c = arranged(&[(1, "HEADLESS-1", rect(-300, 600, 320, 240))]);
    let mut book = known(&a, STANDARD);
    let mut from = &a;
    for (n, to) in [&b, &c, &a, &b, &c].into_iter().enumerate() {
        let t = at(60 * n as u64 + 40);
        book = retarget(book, from, to, &Cause::Keyboard, STANDARD, t);
        from = to;
    }
    assert!(book.is_moving(at(400)));
    let end = drawn(&book, 1, at(10_000));
    assert_eq!(end, Some([-36_000, 72_000, 38_400, 28_800, 1000]));
    assert!(!book.is_moving(at(10_000)));
}

#[test]
fn a_remote_spring_never_overshoots_and_a_throw_may() {
    let a = arranged(&[(1, "HEADLESS-1", rect(0, 0, 400, 300))]);
    let b = arranged(&[(1, "HEADLESS-1", rect(1000, 0, 400, 300))]);
    let peak = |cause: &Cause, level| {
        let book = retarget(known(&a, level), &a, &b, cause, level, at(0));
        (0..2_000)
            .filter_map(|ms| drawn(&book, 1, at(ms)))
            .map(|d| d[0])
            .max()
            .unwrap_or(0)
    };
    let target = 1000 * 120;
    assert!(
        peak(&Cause::Keyboard, STANDARD) <= target,
        "a key does not overshoot"
    );
    assert!(peak(&Cause::Overview, STANDARD) <= target);
    let swipe = Cause::Swipe(SwipeRelease {
        velocity: Velocity(3_000),
    });
    assert!(peak(&swipe, STANDARD) > target, "a hand's throw may");
    assert!(peak(&swipe, REDUCED) <= target, "never under Reduced");
}

#[test]
fn reduced_motion_settles_at_once() {
    let (desktop, overview) = arrangements();
    let causes = [
        Cause::Keyboard,
        Cause::Drop,
        Cause::Overview,
        Cause::SpaceSwitch,
        Cause::Swipe(SwipeRelease {
            velocity: Velocity(2_000),
        }),
    ];
    for cause in causes {
        let book = retarget(
            known(&desktop, REDUCED),
            &desktop,
            &overview,
            &cause,
            REDUCED,
            at(100),
        );
        assert!(!book.is_moving(at(100)), "{cause:?}: nothing moves");
        let (frame, settled) = sample(
            &book,
            &CommittedSizes::default(),
            &WindowLooks::default(),
            at(100),
        );
        assert_eq!(settled, Settled::Rest, "{cause:?}");
        for (n, _, slot) in journeys() {
            let w = placed(&frame, n).expect("drawn");
            assert_eq!(
                (w.at.x, w.at.y, w.size.w, w.size.h),
                (
                    SubPx(slot.origin.x * 120),
                    SubPx(slot.origin.y * 120),
                    SubPx(slot.size.w as i32 * 120),
                    SubPx(slot.size.h as i32 * 120)
                ),
                "{cause:?}: window {n} is in its slot at once"
            );
        }
    }
}

#[test]
fn reduced_arrivals_and_departures_cross_fade_in_place() {
    let empty = arranged(&[]);
    let one = arranged(&[(1, "HEADLESS-1", rect(100, 100, 400, 300))]);
    let arriving = retarget(
        known(&empty, REDUCED),
        &empty,
        &one,
        &Cause::Open,
        REDUCED,
        at(0),
    );
    let samples: Vec<_> = (0..600)
        .filter_map(|ms| drawn(&arriving, 1, at(ms)))
        .collect();
    assert!(
        samples
            .iter()
            .all(|d| d[..4] == [12_000, 12_000, 48_000, 36_000]),
        "never moves or grows"
    );
    assert_eq!(samples[0][4], 0, "starts invisible");
    assert!(
        samples.windows(2).all(|p| p[0][4] <= p[1][4]),
        "fades in monotonically"
    );
    assert_eq!(samples.last().map(|d| d[4]), Some(1000));
    let leaving = retarget(arriving, &one, &empty, &Cause::Close, REDUCED, at(1_000));
    assert!(leaving.is_moving(at(1_000)));
    assert_eq!(
        drawn(&leaving, 1, at(1_000)).map(|d| d[..4].to_vec()),
        Some(vec![12_000, 12_000, 48_000, 36_000])
    );
    assert_eq!(drawn(&leaving, 1, at(5_000)), None, "gone once faded");
    assert!(!leaving.is_moving(at(5_000)));
}

#[test]
fn a_standard_arrival_grows_in_and_a_departure_shrinks_away() {
    let empty = arranged(&[]);
    let one = arranged(&[(1, "HEADLESS-1", rect(100, 100, 1000, 500))]);
    let book = retarget(
        MotionBook::empty(),
        &empty,
        &one,
        &Cause::Open,
        STANDARD,
        at(0),
    );
    let start = drawn(&book, 1, at(0)).expect("drawn at once");
    assert_eq!(start[4], 0, "invisible");
    assert!(
        start[2] < 120_000 * 1000 / 1000 && start[2] == 110_400,
        "92% wide: {start:?}"
    );
    let end = drawn(&book, 1, at(3_000)).expect("drawn");
    assert_eq!(end, [12_000, 12_000, 120_000, 60_000, 1000]);
    let gone = retarget(book, &one, &empty, &Cause::Close, STANDARD, at(3_000));
    let mid = drawn(&gone, 1, at(3_100)).expect("fading");
    assert!(mid[2] < 120_000 && mid[4] < 1000 && mid[4] > 0, "{mid:?}");
    assert_eq!(drawn(&gone, 1, at(9_000)), None);
}

#[test]
fn minimising_travels_to_the_dock_tile_and_unminimising_comes_back_from_it() {
    let tile = rect(900, 1040, 48, 30);
    let target = MinimiseTarget {
        output: output("HEADLESS-1"),
        rect: tile,
    };
    let cause = Cause::Minimise(target);
    let shown = arranged(&[(1, "HEADLESS-1", rect(200, 200, 800, 600))]);
    let hidden = arranged(&[]);
    let down = retarget(
        known(&shown, STANDARD),
        &shown,
        &hidden,
        &cause,
        STANDARD,
        at(0),
    );
    assert_eq!(drawn(&down, 1, at(0)).map(|d| d[4]), Some(1000));
    let late = drawn(&down, 1, at(300)).expect("still fading");
    assert!(
        late[0] > 24_000 && late[0] <= 900 * 120,
        "heads for the tile: {late:?}"
    );
    assert_eq!(drawn(&down, 1, at(9_000)), None, "in the dock");
    let up = retarget(down, &hidden, &shown, &cause, STANDARD, at(10_000));
    let first = drawn(&up, 1, at(10_000)).expect("comes out of the tile");
    assert_eq!(first[..4], [900 * 120, 1040 * 120, 48 * 120, 30 * 120]);
    assert_eq!(first[4], 0);
    assert_eq!(
        drawn(&up, 1, at(19_000)),
        Some([24_000, 24_000, 96_000, 72_000, 1000])
    );
}

#[test]
fn a_minimise_flies_as_a_flight_out_and_back_and_other_causes_do_not() {
    let tile = rect(900, 1040, 48, 30);
    let cause = Cause::Minimise(MinimiseTarget {
        output: output("HEADLESS-1"),
        rect: tile,
    });
    let shown = arranged(&[(1, "HEADLESS-1", rect(200, 200, 800, 600))]);
    let hidden = arranged(&[]);
    let flight_at = |book: &MotionBook, t| {
        let (frame, _) = sample(book, &CommittedSizes::default(), &WindowLooks::default(), t);
        placed(&frame, 1).and_then(|window| window.flight)
    };
    let down = retarget(
        known(&shown, STANDARD),
        &shown,
        &hidden,
        &cause,
        STANDARD,
        at(0),
    );
    let start = flight_at(&down, at(0)).expect("in flight at once");
    assert_eq!((start.window, start.tile), (rect(200, 200, 800, 600), tile));
    assert_eq!(start.progress.0, 0);
    let mid = flight_at(&down, at(150)).expect("still in flight");
    assert!(
        mid.progress.0 > 0 && mid.progress.0 < 1000,
        "between the ends: {mid:?}"
    );
    let late = flight_at(&down, at(300)).expect("late in flight");
    assert!(late.progress.0 > mid.progress.0, "it only gets closer");
    assert_eq!(
        flight_at(&down, at(9_000)),
        None,
        "arrived: nothing to draw"
    );
    let up = retarget(down, &hidden, &shown, &cause, STANDARD, at(10_000));
    let back = flight_at(&up, at(10_000)).expect("leaves the tile");
    assert_eq!(back.window, rect(200, 200, 800, 600));
    assert_eq!(back.progress.0, 1000);
    let later = flight_at(&up, at(10_150)).expect("on its way out");
    assert!(later.progress.0 < 1000, "it only gets farther: {later:?}");
    assert_eq!(flight_at(&up, at(19_000)), None, "back at rest");
    let keyed = retarget(
        known(&shown, STANDARD),
        &shown,
        &hidden,
        &Cause::Keyboard,
        STANDARD,
        at(0),
    );
    assert_eq!(flight_at(&keyed, at(100)), None, "only a minimise flies");
    let reduced = retarget(
        known(&shown, REDUCED),
        &shown,
        &hidden,
        &cause,
        REDUCED,
        at(0),
    );
    assert_eq!(
        flight_at(&reduced, at(100)),
        None,
        "reduced motion does not fly"
    );
}

#[test]
fn a_frame_lists_windows_bottom_first_on_each_output_in_key_order() {
    let a = arranged(&[
        (9, "HEADLESS-2", rect(1920, 0, 100, 100)),
        (7, "HEADLESS-1", rect(0, 0, 100, 100)),
        (3, "HEADLESS-1", rect(10, 10, 100, 100)),
        (5, "HEADLESS-2", rect(1930, 10, 100, 100)),
    ]);
    let (frame, _) = sample(
        &known(&a, STANDARD),
        &CommittedSizes::default(),
        &WindowLooks::default(),
        at(0),
    );
    let order: Vec<(String, Vec<u64>)> = frame
        .outputs
        .iter()
        .map(|o| {
            (
                o.output.as_str().to_owned(),
                o.windows.iter().map(|w| w.window.0).collect(),
            )
        })
        .collect();
    assert_eq!(
        order,
        vec![
            ("connector:HEADLESS-1".to_owned(), vec![7, 3]),
            ("connector:HEADLESS-2".to_owned(), vec![9, 5]),
        ]
    );
}

#[test]
fn a_window_is_drawn_with_its_look_and_its_committed_size() {
    let a = arranged(&[
        (1, "HEADLESS-1", rect(0, 0, 400, 300)),
        (2, "HEADLESS-1", rect(0, 0, 400, 300)),
        (3, "HEADLESS-1", rect(0, 0, 400, 300)),
        (4, "HEADLESS-1", rect(0, 0, 400, 300)),
    ]);
    let looks = WindowLooks(
        [
            (
                WindowId(1),
                look(Emphasis::Key, FrameMode::CompositorShadow, 12),
            ),
            (
                WindowId(2),
                look(Emphasis::Main, FrameMode::CompositorShadow, 12),
            ),
            (WindowId(3), look(Emphasis::Inactive, FrameMode::Client, 0)),
        ]
        .into(),
    );
    let (frame, _) = sample(
        &known(&a, STANDARD),
        &committed(&[(1, 380, 280)]),
        &looks,
        at(0),
    );
    let row = |n| {
        let w = placed(&frame, n).expect("drawn");
        (w.emphasis, w.shadow, w.corner.0, w.committed)
    };
    use casement_motion::ShadowLevel::{Inactive, Key, None};
    assert_eq!(
        row(1),
        (
            Emphasis::Key,
            Key,
            SubPx(1_440),
            LogicalSize { w: 380, h: 280 }
        )
    );
    assert_eq!(
        row(2),
        (
            Emphasis::Main,
            Inactive,
            SubPx(1_440),
            LogicalSize { w: 400, h: 300 }
        )
    );
    assert_eq!(
        row(3),
        (
            Emphasis::Inactive,
            None,
            SubPx(0),
            LogicalSize { w: 400, h: 300 }
        )
    );
    assert_eq!(
        row(4),
        (
            Emphasis::Inactive,
            None,
            SubPx(0),
            LogicalSize { w: 400, h: 300 }
        ),
        "no look: inactive, client frame"
    );
}

/// An arrangement whose output has its Space strip's first Space at `x`.
fn strip_at(x: i32) -> Arrangement {
    Arrangement {
        spaces: vec![
            SpaceTarget {
                space: SpaceId::parse("s-1").expect("id"),
                output: output("HEADLESS-1"),
                origin: LogicalPoint { x, y: 24 },
            },
            SpaceTarget {
                space: SpaceId::parse("s-2").expect("id"),
                output: output("HEADLESS-1"),
                origin: LogicalPoint { x: x + 1920, y: 24 },
            },
        ],
        ..Arrangement::default()
    }
}

fn strip(book: &MotionBook, ms: u64) -> i32 {
    let (frame, _) = sample(
        book,
        &CommittedSizes::default(),
        &WindowLooks::default(),
        at(ms),
    );
    frame.outputs[0].strip.0.0
}

#[test]
fn the_space_strip_slides_with_the_cause_and_stops_exactly() {
    let (home, next) = (strip_at(0), strip_at(-1920));
    let slide = retarget(
        known(&home, STANDARD),
        &home,
        &next,
        &Cause::SpaceSwitch,
        STANDARD,
        at(0),
    );
    assert_eq!(strip(&slide, 0), 0);
    let mid = strip(&slide, 150);
    assert!((-1920 * 120..0).contains(&mid), "{mid}");
    assert_eq!(strip(&slide, 5_000), -1920 * 120);
    assert!(!slide.is_moving(at(5_000)));
    let thrown = Cause::Swipe(SwipeRelease {
        velocity: Velocity(-4_000),
    });
    let flick = retarget(
        known(&home, STANDARD),
        &home,
        &next,
        &thrown,
        STANDARD,
        at(0),
    );
    assert!(
        strip(&flick, 30) < strip(&slide, 30),
        "a throw starts faster"
    );
    for cause in [Cause::OutputChange, Cause::Configure] {
        let snapped = retarget(
            known(&home, STANDARD),
            &home,
            &next,
            &cause,
            STANDARD,
            at(0),
        );
        assert_eq!(
            (strip(&snapped, 0), snapped.is_moving(at(0))),
            (-1920 * 120, false),
            "{cause:?}"
        );
    }
    let reduced = retarget(
        known(&home, REDUCED),
        &home,
        &next,
        &Cause::SpaceSwitch,
        REDUCED,
        at(0),
    );
    assert_eq!(
        (strip(&reduced, 0), reduced.is_moving(at(0))),
        (-1920 * 120, false)
    );
}

/// A swipe that drives an output's row draws it itself (its finish is the model's): the strip
/// leg goes to the new target at once, so a commit does not slide a second time. Another output
/// and the windows keep their own travel.
#[test]
fn no_space_slide_while_a_swipe_drives_the_strip() {
    let (home, next) = (strip_at(0), strip_at(-1920));
    let driven: BTreeSet<_> = [output("HEADLESS-1")].into();
    let held = retarget_driven(
        known(&home, STANDARD),
        &home,
        &next,
        &Cause::SpaceSwitch,
        STANDARD,
        at(0),
        &driven,
    );
    assert_eq!(
        (strip(&held, 0), held.is_moving(at(0))),
        (-1920 * 120, false)
    );
    assert_eq!(strip(&held, 150), -1920 * 120, "it stays there");
    let elsewhere: BTreeSet<_> = [output("HEADLESS-2")].into();
    let own = retarget_driven(
        known(&home, STANDARD),
        &home,
        &next,
        &Cause::SpaceSwitch,
        STANDARD,
        at(0),
        &elsewhere,
    );
    assert_eq!(
        strip(&own, 0),
        0,
        "another output's swipe leaves this slide"
    );
    assert!(own.is_moving(at(0)));
    let plain = retarget(
        known(&home, STANDARD),
        &home,
        &next,
        &Cause::SpaceSwitch,
        STANDARD,
        at(0),
    );
    assert_eq!(strip(&own, 150), strip(&plain, 150));
}

/// The fingers hold Mission Control half open (every move is a snap to the mixed arrangement);
/// a commit then travels from there to the slots with no jump, and a snap-back returns to the
/// desktop the same way.
#[test]
fn a_commit_and_a_snap_back_continue_from_the_followed_progress() {
    let (desktop, overview) = arrangements();
    let half = overview.spread_at(Fraction(500));
    let followed = retarget(
        known(&desktop, STANDARD),
        &desktop,
        &half,
        &Cause::Configure,
        STANDARD,
        at(0),
    );
    assert!(!followed.is_moving(at(0)), "the fingers hold it still");
    let commit = retarget(
        followed.clone(),
        &desktop,
        &overview,
        &Cause::Overview,
        STANDARD,
        at(40),
    );
    let back = retarget(
        followed.clone(),
        &desktop,
        &desktop,
        &Cause::Overview,
        STANDARD,
        at(40),
    );
    for (n, from, slot) in journeys() {
        let held = drawn(&followed, n, at(40));
        assert_eq!(
            drawn(&commit, n, at(40)),
            held,
            "window {n} jumped at the commit"
        );
        assert_eq!(
            drawn(&back, n, at(40)),
            held,
            "window {n} jumped at the snap-back"
        );
        let on = |book: &MotionBook, to: casement_core::geometry::LogicalRect| {
            drawn(book, n, at(5_000)).map(|[x, y, w, h, _]| (x, y, w, h))
                == Some((
                    i64::from(to.origin.x) * 120,
                    i64::from(to.origin.y) * 120,
                    i64::from(to.size.w) * 120,
                    i64::from(to.size.h) * 120,
                ))
        };
        assert!(on(&commit, slot), "window {n} ends in its slot");
        assert!(on(&back, from), "window {n} ends on the desktop");
    }
}

#[test]
fn an_output_that_loses_its_spaces_loses_its_strip() {
    let home = strip_at(0);
    let none = Arrangement::default();
    let book = retarget(
        known(&home, STANDARD),
        &home,
        &none,
        &Cause::OutputChange,
        STANDARD,
        at(10),
    );
    let (frame, settled) = sample(
        &book,
        &CommittedSizes::default(),
        &WindowLooks::default(),
        at(10),
    );
    assert_eq!((frame.outputs.len(), settled), (0, Settled::Rest));
}
