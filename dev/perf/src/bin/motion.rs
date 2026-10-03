//! `sample` and `retarget` timing for 50 windows (the plan's budget: sampling p95 at most 50 us),
//! outside `crates/` because boundary row B8 keeps `Instant::now` there.
//! `motion [ITERATIONS] [--tsv]`.
use casement_core::SwipeRelease;
use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize};
use casement_core::output::{EdidFacts, EdidSerial};
use casement_core::time::Mono;
use casement_core::window::FrameHint;
use casement_core::{Cause, OutputKey, OutputName, WindowId};
use casement_layout::{Arrangement, ConfigureNeed, OverviewSlot, SlotKind, WindowTarget};
use casement_motion::*;
use casement_perf::{Options, time_row};
use casement_protocols::vocab::{Emphasis, FrameMode};
use ds_motion::velocity as casement_motion_velocity;
use ds_style::appearance::motion::MotionLevel;
use std::time::Duration;

fn rect(x: i32, y: i32, w: u32, h: u32) -> LogicalRect {
    LogicalRect::new(LogicalPoint { x, y }, LogicalSize { w, h })
}

fn lcg(s: &mut u64) -> i32 {
    *s = s
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    (*s >> 33) as i32
}

fn scene(n: u64, outputs: u64) -> (Arrangement, Arrangement) {
    let edid = EdidFacts {
        make: String::new(),
        model: String::new(),
        serial: EdidSerial::Unknown,
    };
    let keys: Vec<OutputKey> = (1..=outputs)
        .map(|o| OutputKey::of(&edid, &OutputName::parse(&format!("HEADLESS-{o}")).unwrap()))
        .collect();
    let mut s = 0x5EED_u64;
    let mut before = Arrangement::default();
    let mut after = Arrangement::default();
    for i in 0..n {
        let (w, h) = (320 + lcg(&mut s) % 700, 220 + lcg(&mut s) % 500);
        let (x, y) = (lcg(&mut s) % (1920 - w), 24 + lcg(&mut s) % (1056 - h));
        let output = keys[(i % outputs) as usize].clone();
        before.windows.push(WindowTarget {
            window: WindowId(i + 1),
            output,
            rect: rect(x, y, w as u32, h as u32),
            size: ConfigureNeed::Keep,
        });
        after.overview.push(OverviewSlot {
            window: WindowId(i + 1),
            kind: SlotKind::Live,
            rect: rect(
                40 + (i % 10) as i32 * 186,
                200 + (i / 10) as i32 * 170,
                (w / 4) as u32,
                (h / 4) as u32,
            ),
        });
    }
    after.windows = before.windows.clone();
    (before, after)
}

fn main() {
    let options = Options::from_args();
    for outputs in [1u64, 2] {
        let (before, after) = scene(50, outputs);
        let at = |us: u64| Mono(Duration::from_micros(us));
        let known = retarget(
            MotionBook::empty(),
            &before,
            &before,
            &Cause::Configure,
            MotionLevel::Standard,
            at(0),
        );
        let flying = retarget(
            known.clone(),
            &before,
            &after,
            &Cause::Overview,
            MotionLevel::Standard,
            at(0),
        );
        let looks = WindowLooks(
            (1..=50u64)
                .map(|n| {
                    let frame = FrameHint {
                        mode: FrameMode::CompositorShadow,
                        corner_radius: 10,
                    };
                    let look = WindowLook {
                        emphasis: Emphasis::Inactive,
                        frame,
                    };
                    (WindowId(n), look)
                })
                .collect(),
        );
        let committed = CommittedSizes(
            (1..=50u64)
                .map(|n| (WindowId(n), LogicalSize { w: 400, h: 300 }))
                .collect(),
        );
        let tag = format!("50 windows, {outputs} output(s)");
        // A frame every millisecond across the first 400 ms of a flight.
        let mid_flight = |k: u64| at(1_000 + (k % 400) * 1_000);

        time_row(&format!("sample, mid-flight, {tag}"), options, 500, |k| {
            std::hint::black_box(sample(
                std::hint::black_box(&flying),
                &committed,
                &looks,
                mid_flight(k),
            ));
        });
        time_row(&format!("sample, all at rest, {tag}"), options, 500, |_| {
            std::hint::black_box(sample(
                std::hint::black_box(&flying),
                &committed,
                &looks,
                at(9_000_000),
            ));
        });
        let swipe = Cause::Swipe(SwipeRelease {
            velocity: casement_motion_velocity::Velocity(900),
        });
        let thrown = retarget(
            known.clone(),
            &before,
            &after,
            &swipe,
            MotionLevel::Standard,
            at(0),
        );
        time_row(
            &format!("sample, thrown (0.8 springs), {tag}"),
            options,
            500,
            |k| {
                std::hint::black_box(sample(
                    std::hint::black_box(&thrown),
                    &committed,
                    &looks,
                    mid_flight(k),
                ));
            },
        );
        time_row(
            &format!("retarget mid-flight (incl. book clone), {tag}"),
            options,
            500,
            |k| {
                std::hint::black_box(retarget(
                    flying.clone(),
                    &after,
                    &before,
                    &Cause::Overview,
                    MotionLevel::Standard,
                    mid_flight(k),
                ));
            },
        );
    }
}
