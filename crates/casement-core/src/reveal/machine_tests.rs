//! The reveal machine, step by step on a script of (instant, input) pairs.

use super::{
    Cover, Place, Popup, Reduced, Reveal, RevealEdge, RevealIn, RevealOut, RevealTiming, Stacking,
};
use crate::time::Mono;
use casement_protocols::vocab::Shown;
use std::time::Duration;

fn at(ms: u64) -> Mono {
    Mono(Duration::from_millis(ms))
}

/// The machine after the script, and the outputs it gave at each step (instant, output).
fn run(
    edge: RevealEdge,
    reduced: Reduced,
    script: &[(u64, RevealIn)],
) -> (Reveal, Vec<(u64, RevealOut)>) {
    let timing = RevealTiming::of(edge, reduced);
    script.iter().fold(
        (Reveal::new(), Vec::new()),
        |(machine, mut seen), (ms, input)| {
            let (next, out) = machine.step(timing, *input, at(*ms));
            seen.extend(out.into_iter().map(|out| (*ms, out)));
            (next, seen)
        },
    )
}

/// A script of (instant, input) pairs.
type Script = Vec<(u64, RevealIn)>;

/// What a script gave, as (instant, output) pairs.
type Told = Vec<(u64, RevealOut)>;

const COVERED: RevealIn = RevealIn::Cover(Cover::Yes);
const AT_EDGE: RevealIn = RevealIn::Pointer(Place::InStrip);
const AWAY: RevealIn = RevealIn::Pointer(Place::Away);
const ON_SURFACE: RevealIn = RevealIn::Pointer(Place::OnSurface);

fn arm(ms: u64) -> RevealOut {
    RevealOut::Arm(at(ms))
}

const SHOWN: RevealOut = RevealOut::Tell(Shown::Shown);
const HIDDEN: RevealOut = RevealOut::Tell(Shown::Hidden);

#[test]
fn the_dwell_fires_at_the_bar_delay_and_not_before() {
    // name, the script, the stacking after it, the outputs
    let cases: [(&str, Script, Stacking, Told); 4] = [
        (
            "arrives",
            vec![(0, COVERED), (1000, AT_EDGE)],
            Stacking::Behind,
            vec![(1000, arm(1450))],
        ),
        (
            "one millisecond short",
            vec![(0, COVERED), (1000, AT_EDGE), (1449, RevealIn::Tick)],
            Stacking::Behind,
            vec![(1000, arm(1450))],
        ),
        (
            "on the dot",
            vec![(0, COVERED), (1000, AT_EDGE), (1450, RevealIn::Tick)],
            Stacking::Above,
            vec![(1000, arm(1450)), (1450, SHOWN), (1450, arm(1650))],
        ),
        (
            "moving inside the strip is still resting",
            vec![
                (0, COVERED),
                (1000, AT_EDGE),
                (1200, AT_EDGE),
                (1450, RevealIn::Tick),
            ],
            Stacking::Above,
            vec![(1000, arm(1450)), (1450, SHOWN), (1450, arm(1650))],
        ),
    ];
    for (name, script, stacking, outputs) in cases {
        let (machine, seen) = run(RevealEdge::Top, Reduced::No, &script);
        assert_eq!(machine.stacking(), stacking, "{name}");
        assert_eq!(seen, outputs, "{name}");
    }
}

#[test]
fn leaving_the_strip_cancels_the_wait() {
    let script = [
        (0, COVERED),
        (1000, AT_EDGE),
        (1200, AWAY),
        (1450, RevealIn::Tick),
    ];
    let (machine, seen) = run(RevealEdge::Top, Reduced::No, &script);
    assert_eq!(machine.stacking(), Stacking::Behind);
    assert_eq!(seen, [(1000, arm(1450)), (1200, RevealOut::Cancel)]);
}

#[test]
fn outside_full_screen_nothing_is_ever_asked() {
    let script = [(1000, AT_EDGE), (1450, RevealIn::Tick), (2000, AWAY)];
    let (machine, seen) = run(RevealEdge::Top, Reduced::No, &script);
    assert_eq!(machine.stacking(), Stacking::Behind);
    assert!(seen.is_empty(), "{seen:?}");
}

#[test]
fn a_pointer_already_at_the_edge_when_full_screen_begins_rests_from_then() {
    let script = [(1000, AT_EDGE), (5000, COVERED), (5450, RevealIn::Tick)];
    let (machine, seen) = run(RevealEdge::Top, Reduced::No, &script);
    assert_eq!(machine.stacking(), Stacking::Above);
    assert_eq!(seen, [(5000, arm(5450)), (5450, SHOWN), (5450, arm(5650))]);
}

/// The script that brings the bar in and leaves it shown with the pointer on it.
fn shown_bar() -> Vec<(u64, RevealIn)> {
    vec![
        (0, COVERED),
        (1000, AT_EDGE),
        (1450, RevealIn::Tick),
        (1650, RevealIn::Tick),
        (1700, ON_SURFACE),
    ]
}

#[test]
fn it_goes_when_the_pointer_leaves_and_drops_behind_the_window_after_the_slide() {
    let mut script = shown_bar();
    script.push((3000, AWAY));
    let (machine, seen) = run(RevealEdge::Top, Reduced::No, &script);
    assert_eq!(machine.stacking(), Stacking::Above, "sliding away");
    assert_eq!(
        seen.last(),
        Some(&(3000, arm(3200))),
        "the slide timed, nothing told yet"
    );

    script.push((3100, RevealIn::Tick));
    let (machine, _) = run(RevealEdge::Top, Reduced::No, &script);
    assert_eq!(machine.stacking(), Stacking::Above, "still on its way");

    script.push((3200, RevealIn::Tick));
    let (machine, seen) = run(RevealEdge::Top, Reduced::No, &script);
    assert_eq!(machine.stacking(), Stacking::Behind, "below the window");
    assert_eq!(seen.last(), Some(&(3200, HIDDEN)), "told when it is gone");
}

#[test]
fn an_open_popup_keeps_it_up_until_it_closes() {
    let mut script = shown_bar();
    script.extend([
        (2000, RevealIn::Popup(Popup::Open)),
        (3000, AWAY),
        (4000, RevealIn::Tick),
    ]);
    let (machine, seen) = run(RevealEdge::Top, Reduced::No, &script);
    assert_eq!(machine.stacking(), Stacking::Above);
    assert!(!seen.iter().any(|(_, out)| *out == HIDDEN), "{seen:?}");

    script.push((5000, RevealIn::Popup(Popup::Closed)));
    let (machine, seen) = run(RevealEdge::Top, Reduced::No, &script);
    assert_eq!(machine.stacking(), Stacking::Above, "sliding away");
    assert_eq!(seen.last(), Some(&(5000, arm(5200))), "the slide timed");

    script.push((5200, RevealIn::Tick));
    let (_, seen) = run(RevealEdge::Top, Reduced::No, &script);
    assert_eq!(seen.last(), Some(&(5200, HIDDEN)), "told when it is gone");
}

#[test]
fn the_pointer_coming_back_while_it_goes_turns_it_round_from_where_it_is() {
    let timing = RevealTiming::of(RevealEdge::Top, Reduced::No);
    let mut script = shown_bar();
    script.extend([(3000, AWAY), (3100, ON_SURFACE)]);
    let (machine, seen) = run(RevealEdge::Top, Reduced::No, &script);
    assert_eq!(machine.stacking(), Stacking::Above);
    assert_eq!(
        seen[seen.len() - 2..],
        [(3000, arm(3200)), (3100, arm(3300))],
        "the shell still holds shown, so it hears nothing"
    );
    let back = machine.visible(timing, at(3100));
    assert!(
        back.0 > 0 && back.0 < 1000,
        "it comes in from half way: {back:?}"
    );
}

#[test]
fn the_dock_keeps_its_own_numbers() {
    let timing = RevealTiming::of(RevealEdge::Bottom, Reduced::No);
    assert_eq!(
        (timing.dwell, timing.travel),
        (Duration::from_millis(200), Duration::from_millis(500))
    );
    let script = [(0, COVERED), (1000, AT_EDGE), (1199, RevealIn::Tick)];
    let (machine, _) = run(RevealEdge::Bottom, Reduced::No, &script);
    assert_eq!(machine.stacking(), Stacking::Behind, "199 ms is not a rest");

    let script = [(0, COVERED), (1000, AT_EDGE), (1200, RevealIn::Tick)];
    let (machine, seen) = run(RevealEdge::Bottom, Reduced::No, &script);
    assert_eq!(machine.stacking(), Stacking::Above);
    assert_eq!(
        seen,
        [(1000, arm(1200)), (1200, SHOWN), (1200, arm(1700))],
        "the slide is the dock's half second"
    );
}

#[test]
fn reduced_motion_keeps_the_dwell_and_fades_for_the_quick_duration() {
    let script = [(0, COVERED), (1000, AT_EDGE), (1450, RevealIn::Tick)];
    let (_, seen) = run(RevealEdge::Top, Reduced::Yes, &script);
    assert_eq!(seen, [(1000, arm(1450)), (1450, SHOWN), (1450, arm(1570))]);
}

#[test]
fn it_comes_in_along_the_out_curve_and_is_whole_when_the_slide_ends() {
    let timing = RevealTiming::of(RevealEdge::Top, Reduced::No);
    let script = [(0, COVERED), (1000, AT_EDGE), (1450, RevealIn::Tick)];
    let (machine, _) = run(RevealEdge::Top, Reduced::No, &script);
    let seen: Vec<u16> = [1450, 1500, 1550, 1600, 1650, 1900]
        .into_iter()
        .map(|ms| machine.visible(timing, at(ms)).0)
        .collect();
    assert_eq!(seen[0], 0);
    assert!(seen.windows(2).all(|pair| pair[0] <= pair[1]), "{seen:?}");
    assert!(
        seen[2] > 500,
        "an ease-out is past half by half way: {seen:?}"
    );
    assert_eq!(seen[4], 1000);
    assert_eq!(seen[5], 1000);
}

#[test]
fn leaving_full_screen_puts_it_away_at_once() {
    // name, the script, what is told at the end
    let cases: [(&str, Script, Vec<RevealOut>); 3] = [
        (
            "shown",
            vec![(2000, RevealIn::Cover(Cover::No))],
            vec![RevealOut::Cancel, HIDDEN],
        ),
        (
            "waiting",
            vec![(1200, RevealIn::Cover(Cover::No))],
            vec![RevealOut::Cancel],
        ),
        (
            "already going",
            vec![(3000, AWAY), (3100, RevealIn::Cover(Cover::No))],
            vec![RevealOut::Cancel, HIDDEN],
        ),
    ];
    for (name, tail, told) in cases {
        let mut script = match name {
            "waiting" => vec![(0, COVERED), (1000, AT_EDGE)],
            _ => shown_bar(),
        };
        let before = run(RevealEdge::Top, Reduced::No, &script).1.len();
        script.extend(tail);
        let (machine, seen) = run(RevealEdge::Top, Reduced::No, &script);
        let after: Vec<RevealOut> = seen
            .iter()
            .skip(before)
            .map(|(_, out)| *out)
            .filter(|out| !matches!(out, RevealOut::Arm(_)))
            .collect();
        assert_eq!(machine.stacking(), Stacking::Behind, "{name}");
        assert_eq!(after.last().copied(), told.last().copied(), "{name}");
        assert!(after.ends_with(&told), "{name}: {after:?}");
    }
}
