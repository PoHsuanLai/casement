//! The S-L8 scene: Mission Control opens over 30 windows. Each travels from its desktop
//! rectangle to its overview slot (scale 0.18 to 0.28; the slots are a table drawn from a
//! 64-bit LCG, since layout is not under test here), and every fifth window is a client-framed
//! one. Windows 1 and 2 are key and main.

use super::{arranged, at, look, rect, with_slots};
use casement_core::geometry::LogicalRect;
use casement_core::{Cause, WindowId};
use casement_layout::Arrangement;
use casement_motion::{CommittedSizes, MotionBook, WindowLooks, retarget};
use casement_protocols::vocab::{Emphasis, FrameMode};
use ds_style::appearance::motion::MotionLevel;

/// The instants S-L8 samples, in milliseconds after the entry starts.
pub const INSTANTS_MS: [u64; 5] = [0, 50, 100, 200, 400];

/// The windows in the scene.
pub const WINDOWS: u64 = 30;

fn lcg(state: &mut u64) -> i32 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    (*state >> 33) as i32
}

/// Each window's desktop rectangle and overview slot.
pub fn journeys() -> Vec<(u64, LogicalRect, LogicalRect)> {
    let mut s = 0x5EED_u64;
    (0..WINDOWS)
        .map(|i| {
            let (w, h) = (320 + lcg(&mut s) % 700, 220 + lcg(&mut s) % 500);
            let (x, y) = (lcg(&mut s) % (1920 - w), 24 + lcg(&mut s) % (1056 - h));
            let k = 0.18 + f64::from(lcg(&mut s) % 100) / 1000.0;
            let (ow, oh) = ((f64::from(w) * k).round(), (f64::from(h) * k).round());
            let ox = 40 + (i % 10) as i32 * 186 + lcg(&mut s) % 3;
            let oy = 200 + (i / 10) as i32 * 170;
            (
                i + 1,
                rect(x, y, w as u32, h as u32),
                rect(ox, oy, ow as u32, oh as u32),
            )
        })
        .collect()
}

/// The desktop arrangement before the entry, and the one with every window in its slot.
pub fn arrangements() -> (Arrangement, Arrangement) {
    let rows: Vec<_> = journeys();
    let desktop: Vec<(u64, &str, LogicalRect)> = rows
        .iter()
        .map(|(n, from, _)| (*n, "HEADLESS-1", *from))
        .collect();
    let slots: Vec<(u64, LogicalRect)> = rows.iter().map(|(n, _, slot)| (*n, *slot)).collect();
    let before = arranged(&desktop);
    let after = with_slots(before.clone(), &slots);
    (before, after)
}

/// Windows 1 and 2 are key and main, every fifth draws its own frame.
pub fn looks_of_scene() -> WindowLooks {
    WindowLooks(
        (1..=WINDOWS)
            .map(|n| {
                let emphasis = match n {
                    1 => Emphasis::Key,
                    2 => Emphasis::Main,
                    _ => Emphasis::Inactive,
                };
                let mode = match n % 5 {
                    0 => FrameMode::Client,
                    _ => FrameMode::CompositorShadow,
                };
                let radius = match mode {
                    FrameMode::Client => 0,
                    FrameMode::CompositorShadow => 10,
                };
                (WindowId(n), look(emphasis, mode, radius))
            })
            .collect(),
    )
}

/// The buffers the clients have committed: their desktop sizes (none has answered the entry).
pub fn committed_of_scene() -> CommittedSizes {
    CommittedSizes(
        journeys()
            .iter()
            .map(|(n, from, _)| (WindowId(*n), from.size))
            .collect(),
    )
}

/// The book the instant Mission Control opens at 0 ms under `level`: every window already known
/// at its desktop place, then aimed at its slot.
pub fn entry(level: MotionLevel) -> MotionBook {
    let (before, after) = arrangements();
    let known = retarget(
        MotionBook::empty(),
        &before,
        &before,
        &Cause::Configure,
        level,
        at(0),
    );
    retarget(known, &before, &after, &Cause::Overview, level, at(0))
}
