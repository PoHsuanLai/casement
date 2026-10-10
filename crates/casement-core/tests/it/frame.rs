//! The frame scheduler's maths (P1-D3). Ignored until the lane fills the functions.

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
#[ignore = "P1-D3"]
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
#[ignore = "P1-D3"]
fn a_frame_that_cannot_make_the_next_vblank_is_for_the_one_after() {
    let clock = VblankClock::new(Millihertz(60_000), ms(0));
    let deadline = next_deadline(&clock, Duration::from_millis(20), ms(1), Duration::ZERO);
    assert_eq!(
        deadline.vblank,
        clock.next_after(clock.next_after(ms(1)).0).0
    );
}

#[test]
#[ignore = "P1-D3"]
fn the_prediction_follows_the_recent_frames() {
    let history = [4_u64, 4, 4, 9]
        .into_iter()
        .fold(RenderHistory::empty(), |h, n| {
            h.with(Duration::from_millis(n))
        });
    assert!(predict(&history) >= Duration::from_millis(4));
}

#[test]
#[ignore = "P1-D3"]
fn judging_a_frame() {
    assert_eq!(judge(VblankSeq(5), VblankSeq(5)), FrameOutcome::OnTime);
    assert_eq!(
        judge(VblankSeq(5), VblankSeq(7)),
        FrameOutcome::Missed { by: 2 }
    );
}

#[test]
#[ignore = "P1-D3"]
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
