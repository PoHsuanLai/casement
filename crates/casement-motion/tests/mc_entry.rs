//! S-L8: the Mission Control entry as goldens (FrameScene text at 0, 50, 100, 200 and 400 ms),
//! and the properties that make a golden trustworthy: independent of how often it is sampled,
//! stable run to run, and a microsecond of timing moves no value by more than one sub-pixel.
//! Goldens change only with a reason in the commit: `CASEMENT_BLESS=1` writes them.

mod support;

use casement_core::time::Mono;
use casement_motion::{FrameScene, MotionBook, Settled, sample};
use ds_style::appearance::motion::MotionLevel;
use std::path::PathBuf;
use support::mc_entry::{INSTANTS_MS, committed_of_scene, entry, journeys, looks_of_scene};
use support::{at, at_us};

fn frame(book: &MotionBook, t: Mono) -> (FrameScene, Settled) {
    sample(book, &committed_of_scene(), &looks_of_scene(), t)
}

/// One block per instant: a header line, then the frame's own text.
fn dump(book: &MotionBook) -> String {
    INSTANTS_MS
        .iter()
        .map(|ms| {
            let (scene, settled) = frame(book, at(*ms));
            format!("t={ms}ms {settled:?}\n{}", scene.text())
        })
        .collect()
}

fn golden_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/goldens")
        .join(name)
}

/// Compares `got` with the golden `name`, writing it first under `CASEMENT_BLESS=1`.
fn assert_golden(name: &str, got: &str) {
    let path = golden_path(name);
    if std::env::var_os("CASEMENT_BLESS").is_some() {
        std::fs::write(&path, got).expect("write the golden");
    }
    let want = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{}: {e}; bless once with CASEMENT_BLESS=1", path.display()));
    assert_eq!(got, want, "{name} differs; bless only with a reason");
}

#[test]
fn the_mission_control_entry_matches_its_golden() {
    assert_golden("mc_entry.txt", &dump(&entry(MotionLevel::Standard)));
}

#[test]
fn the_reduced_entry_matches_its_golden() {
    assert_golden("mc_entry_reduced.txt", &dump(&entry(MotionLevel::Reduced)));
}

/// The golden is not only a recording: its ends are the table's own numbers.
#[test]
fn the_entry_starts_at_the_desktop_and_ends_at_the_slots() {
    let book = entry(MotionLevel::Standard);
    let rows = journeys();
    let sub = |p: casement_core::geometry::SubPx| i64::from(p.0);
    let (first, moving) = frame(&book, at(0));
    assert_eq!(moving, Settled::Moving);
    let (last, rest) = frame(&book, at(5_000));
    assert_eq!(rest, Settled::Rest);
    let windows = |scene: &FrameScene| scene.outputs[0].windows.clone();
    for ((n, from, slot), (a, z)) in rows.iter().zip(windows(&first).iter().zip(windows(&last))) {
        assert_eq!((a.window.0, z.window.0), (*n, *n));
        assert_eq!(
            [sub(a.at.x), sub(a.at.y), sub(a.size.w), sub(a.size.h)],
            [
                i64::from(from.origin.x) * 120,
                i64::from(from.origin.y) * 120,
                i64::from(from.size.w) * 120,
                i64::from(from.size.h) * 120
            ],
            "window {n} starts at its desktop place"
        );
        assert_eq!(
            [sub(z.at.x), sub(z.at.y), sub(z.size.w), sub(z.size.h)],
            [
                i64::from(slot.origin.x) * 120,
                i64::from(slot.origin.y) * 120,
                i64::from(slot.size.w) * 120,
                i64::from(slot.size.h) * 120
            ],
            "window {n} ends in its slot"
        );
        assert_eq!(z.committed, from.size, "the buffer is still the old one");
    }
}

#[test]
fn sampling_never_leaves_a_trace() {
    let first = dump(&entry(MotionLevel::Standard));
    for _ in 0..20 {
        assert_eq!(dump(&entry(MotionLevel::Standard)), first);
    }
    let book = entry(MotionLevel::Standard);
    for ms in 0..400 {
        let _ = frame(&book, at(ms));
    }
    assert_eq!(dump(&book), first, "frame cadence does not matter");
}

fn largest_shift(book: &MotionBook, t: Mono, by_us: u64) -> i32 {
    let (a, _) = frame(book, t);
    let (b, _) = frame(book, Mono(t.0 + std::time::Duration::from_micros(by_us)));
    let pairs = a.outputs[0].windows.iter().zip(&b.outputs[0].windows);
    pairs
        .flat_map(|(p, q)| {
            [
                (p.at.x.0 - q.at.x.0).abs(),
                (p.at.y.0 - q.at.y.0).abs(),
                (p.size.w.0 - q.size.w.0).abs(),
                (p.size.h.0 - q.size.h.0).abs(),
                (i32::from(p.opacity.0) - i32::from(q.opacity.0)).abs(),
            ]
        })
        .max()
        .unwrap_or(0)
}

/// The libm risk of the goldens (A25): a microsecond's shift in the clock moves a value by at
/// most one sub-pixel, so a one-ulp difference in `exp` cannot move a golden more than that.
#[test]
fn a_microsecond_moves_no_value_by_more_than_one_sub_pixel() {
    let book = entry(MotionLevel::Standard);
    let at_instants = INSTANTS_MS
        .iter()
        .map(|ms| largest_shift(&book, at(*ms), 1))
        .max();
    let sweep = (0..800_000u64)
        .step_by(997)
        .map(|us| largest_shift(&book, at_us(us), 1))
        .max();
    assert!(at_instants <= Some(1), "golden instants: {at_instants:?}");
    assert!(sweep <= Some(1), "sweep over 0 to 800 ms: {sweep:?}");
}
