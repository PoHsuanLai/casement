//! The frame scheduler's maths (P1-D3). Pure maths, no clock.

use casement_core::frame::{
    CursorPlaneCaps, CursorPlaneChoice, FrameOutcome, RenderHistory, cursor_plane_for, judge,
    next_deadline, predict,
};
use casement_core::geometry::PhysicalSize;
use casement_core::time::{Millihertz, Mono, VblankClock, VblankSeq};
use ds_core::geometry::scale::Scale;
use std::time::Duration;

fn ms(n: u64) -> Mono {
    Mono(Duration::from_millis(n))
}

#[test]
fn a_cheap_frame_starts_late_enough_and_ends_on_the_next_vblank() {
    let clock = VblankClock::new(Millihertz(60_000), ms(0));
    let deadline = next_deadline(
        &clock,
        Duration::from_millis(4),
        ms(1),
        Duration::from_millis(1),
    );
    assert_eq!(deadline.vblank, clock.next_after(ms(1)).0);
    assert_eq!(
        deadline.render_at,
        Mono(deadline.vblank.0 - Duration::from_millis(5)),
        "predicted cost plus slack before the vblank"
    );
}

#[test]
fn a_frame_that_cannot_make_the_next_vblank_is_for_the_one_after() {
    let clock = VblankClock::new(Millihertz(60_000), ms(0));
    let deadline = next_deadline(&clock, Duration::from_millis(20), ms(1), Duration::ZERO);
    assert_eq!(
        deadline.vblank,
        clock.next_after(clock.next_after(ms(1)).0).0
    );
}

#[test]
fn the_prediction_follows_the_recent_frames() {
    let history = [4_u64, 4, 4, 9]
        .into_iter()
        .fold(RenderHistory::empty(), |h, n| {
            h.with(Duration::from_millis(n))
        });
    assert!(predict(&history) >= Duration::from_millis(4));
}

#[test]
fn judging_a_frame() {
    assert_eq!(judge(VblankSeq(5), VblankSeq(5)), FrameOutcome::OnTime);
    assert_eq!(
        judge(VblankSeq(5), VblankSeq(7)),
        FrameOutcome::Missed { by: 2 }
    );
}

#[test]
fn the_cursor_rides_a_plane_only_when_it_fits() {
    let caps = CursorPlaneCaps::new(Some(PhysicalSize { w: 64, h: 64 }));
    let at = Scale(120);
    assert_eq!(
        cursor_plane_for(&caps, PhysicalSize { w: 32, h: 32 }, at),
        CursorPlaneChoice::Hardware
    );
    assert_eq!(
        cursor_plane_for(&caps, PhysicalSize { w: 128, h: 128 }, at),
        CursorPlaneChoice::Composited
    );
    assert_eq!(
        cursor_plane_for(
            &CursorPlaneCaps::new(None),
            PhysicalSize { w: 32, h: 32 },
            at
        ),
        CursorPlaneChoice::Composited
    );
}

#[test]
fn the_prediction_is_the_worst_of_the_last_eight_and_empty_is_conservative() {
    let history = (1..=20_u64).fold(RenderHistory::empty(), |h, n| {
        h.with(Duration::from_millis(n))
    });
    assert_eq!(history.costs().len(), 8);
    assert_eq!(history.costs().first(), Some(&Duration::from_millis(13)));
    assert_eq!(predict(&history), Duration::from_millis(20));
    assert_eq!(predict(&RenderHistory::empty()), Duration::from_millis(8));
}

#[test]
fn a_frame_shown_early_is_on_time() {
    assert_eq!(judge(VblankSeq(5), VblankSeq(4)), FrameOutcome::OnTime);
}

#[test]
fn a_fractional_scale_grows_the_cursor_before_the_fit_test() {
    let caps = CursorPlaneCaps::new(Some(PhysicalSize { w: 64, h: 64 }));
    let size = PhysicalSize { w: 40, h: 40 };
    assert_eq!(
        cursor_plane_for(&caps, size, Scale(120)),
        CursorPlaneChoice::Hardware
    );
    assert_eq!(
        cursor_plane_for(&caps, size, Scale(240)),
        CursorPlaneChoice::Composited,
        "40 logical px at 2x is 80 physical"
    );
}

#[test]
fn a_repeating_fault_is_reported_once_per_window_with_the_count_held_back() {
    use casement_core::frame::{Repeat, RepeatGate};
    let mut gate = RepeatGate::new(Duration::from_secs(30));
    let at = |s: u64| Mono(Duration::from_secs(s));
    let steps = [
        ("a", 0, Repeat::Report { suppressed: 0 }),
        ("a", 1, Repeat::Quiet),
        ("b", 1, Repeat::Report { suppressed: 0 }),
        ("a", 29, Repeat::Quiet),
        ("a", 30, Repeat::Report { suppressed: 2 }),
        ("a", 31, Repeat::Quiet),
        ("a", 61, Repeat::Report { suppressed: 1 }),
    ];
    for (key, second, want) in steps {
        assert_eq!(gate.admit(key, at(second)), want, "{key} at {second}s");
    }
    gate.clear(&"a");
    assert_eq!(
        gate.admit("a", at(62)),
        Repeat::Report { suppressed: 0 },
        "a fault after a good frame is a first one"
    );
}
