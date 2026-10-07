//! Retargeting and sampling (the behaviour P0-F1-motion makes pass): motion ends exactly at
//! its targets, a retarget keeps position and velocity, frames are deterministic.

mod support;

use casement_core::geometry::{LogicalSize, SubPx};
use casement_core::{Cause, WindowId};
use casement_motion::{FrameScene, MotionBook, Settled, retarget, sample};
use ds_style::appearance::motion::MotionLevel;
use std::time::Duration;
use support::{arranged, at, committed, looks, output, placed, rect};

/// Where window `n` is drawn at `t`, in sub-pixels.
fn x_at(book: &MotionBook, n: u64, t: casement_core::time::Mono) -> i64 {
    let (frame, _) = sample(book, &committed(&[(n, 400, 300)]), &looks(&[n]), t);
    i64::from(placed(&frame, n).expect("window drawn").at.x.0)
}

#[test]
fn an_unchanged_arrangement_does_not_move() {
    let a = arranged(&[(1, "HEADLESS-1", rect(100, 100, 400, 300))]);
    let book = retarget(
        MotionBook::empty(),
        &a,
        &a,
        &Cause::Configure,
        MotionLevel::Standard,
        at(0),
    );
    assert!(!book.is_moving(at(0)), "nothing changed, nothing moves");
    let (frame, settled) = sample(&book, &committed(&[(1, 400, 300)]), &looks(&[1]), at(0));
    assert_eq!(settled, Settled::Rest);
    let w = placed(&frame, 1).expect("drawn");
    assert_eq!(
        (w.at.x, w.at.y, w.size.w, w.size.h),
        (SubPx(12_000), SubPx(12_000), SubPx(48_000), SubPx(36_000))
    );
}

#[test]
fn a_move_starts_where_it_was_and_ends_exactly_at_its_target() {
    let a = arranged(&[(1, "HEADLESS-1", rect(100, 100, 400, 300))]);
    let b = arranged(&[(1, "HEADLESS-1", rect(900, 500, 400, 300))]);
    let book = retarget(
        MotionBook::empty(),
        &a,
        &a,
        &Cause::Configure,
        MotionLevel::Standard,
        at(0),
    );
    let book = retarget(
        book,
        &a,
        &b,
        &Cause::Keyboard,
        MotionLevel::Standard,
        at(10),
    );
    assert!(book.is_moving(at(10)));
    assert_eq!(x_at(&book, 1, at(10)), 12_000, "starts at the old place");
    let (frame, settled) = sample(&book, &committed(&[(1, 400, 300)]), &looks(&[1]), at(3_000));
    assert_eq!(settled, Settled::Rest);
    assert!(!book.is_moving(at(3_000)));
    let w = placed(&frame, 1).expect("drawn");
    assert_eq!((w.at.x, w.at.y), (SubPx(108_000), SubPx(60_000)));
}

#[test]
fn a_retarget_keeps_position_and_velocity() {
    let a = arranged(&[(1, "HEADLESS-1", rect(0, 0, 400, 300))]);
    let b = arranged(&[(1, "HEADLESS-1", rect(4_000, 0, 400, 300))]);
    let c = arranged(&[(1, "HEADLESS-1", rect(-4_000, 0, 400, 300))]);
    let start = retarget(
        MotionBook::empty(),
        &a,
        &a,
        &Cause::Configure,
        MotionLevel::Standard,
        at(0),
    );
    let first = retarget(
        start,
        &a,
        &b,
        &Cause::Keyboard,
        MotionLevel::Standard,
        at(0),
    );
    let t1 = at(80);
    let second = retarget(
        first.clone(),
        &b,
        &c,
        &Cause::Keyboard,
        MotionLevel::Standard,
        t1,
    );
    assert_eq!(
        x_at(&first, 1, t1),
        x_at(&second, 1, t1),
        "position is continuous"
    );
    // Velocity is read over a step short enough that the spring's acceleration, which the
    // retarget does change (the target moved), adds less than the 1% tolerance: the error is
    // half the acceleration step times the step, so it shrinks with the step.
    let step = Duration::from_micros(25);
    let before = x_at(&first, 1, t1.after(step)) - x_at(&first, 1, t1);
    let after = x_at(&second, 1, t1.after(step)) - x_at(&second, 1, t1);
    assert!(
        before.abs() > 40,
        "the window is moving fast at t1 ({before} sub-px per step)"
    );
    let tolerance = before.abs() / 100 + 2;
    assert!(
        (before - after).abs() <= tolerance,
        "velocity kept within 1%: {before} then {after}"
    );
}

#[test]
fn reduced_motion_still_arrives() {
    let a = arranged(&[(1, "HEADLESS-1", rect(100, 100, 400, 300))]);
    let b = arranged(&[(1, "HEADLESS-1", rect(600, 100, 500, 300))]);
    let book = retarget(
        MotionBook::empty(),
        &a,
        &a,
        &Cause::Configure,
        MotionLevel::Reduced,
        at(0),
    );
    let book = retarget(book, &a, &b, &Cause::Drag, MotionLevel::Reduced, at(0));
    let (frame, settled) = sample(&book, &committed(&[(1, 500, 300)]), &looks(&[1]), at(2_000));
    assert_eq!(settled, Settled::Rest);
    let w = placed(&frame, 1).expect("drawn");
    assert_eq!(
        (w.at.x, w.size.w, w.committed),
        (SubPx(72_000), SubPx(60_000), LogicalSize { w: 500, h: 300 })
    );
}

#[test]
fn windows_open_and_close_and_group_by_output() {
    let empty = arranged(&[]);
    let two = arranged(&[
        (1, "HEADLESS-1", rect(0, 0, 400, 300)),
        (2, "HEADLESS-2", rect(1920, 0, 400, 300)),
    ]);
    let book = retarget(
        MotionBook::empty(),
        &empty,
        &two,
        &Cause::Open,
        MotionLevel::Standard,
        at(0),
    );
    let (frame, _) = sample(
        &book,
        &committed(&[(1, 400, 300), (2, 400, 300)]),
        &looks(&[1, 2]),
        at(3_000),
    );
    let on = |name: &str| -> Vec<WindowId> {
        frame
            .outputs
            .iter()
            .filter(|o| o.output == output(name))
            .flat_map(|o| o.windows.iter().map(|w| w.window))
            .collect()
    };
    assert_eq!(
        (on("HEADLESS-1"), on("HEADLESS-2")),
        (vec![WindowId(1)], vec![WindowId(2)])
    );
    assert_eq!(placed(&frame, 1).map(|w| w.opacity.0), Some(1000));
    let one = arranged(&[(1, "HEADLESS-1", rect(0, 0, 400, 300))]);
    let book = retarget(
        book,
        &two,
        &one,
        &Cause::Close,
        MotionLevel::Standard,
        at(3_000),
    );
    let (frame, settled) = sample(&book, &committed(&[(1, 400, 300)]), &looks(&[1]), at(6_000));
    assert_eq!(settled, Settled::Rest);
    assert_eq!(
        placed(&frame, 2),
        None,
        "a closed window leaves once it has faded"
    );
}

#[test]
fn sampling_is_deterministic() {
    let a = arranged(&[(1, "HEADLESS-1", rect(0, 0, 400, 300))]);
    let b = arranged(&[(1, "HEADLESS-1", rect(700, 300, 600, 400))]);
    let book = retarget(
        MotionBook::empty(),
        &a,
        &b,
        &Cause::Drag,
        MotionLevel::Standard,
        at(0),
    );
    let frames = |book: &MotionBook| -> Vec<(FrameScene, Settled)> {
        (0..60u64)
            .map(|n| sample(book, &committed(&[(1, 400, 300)]), &looks(&[1]), at(n * 16)))
            .collect()
    };
    assert_eq!(frames(&book), frames(&book.clone()));
}
