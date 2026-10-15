//! Builders shared by the motion tests (not API).

use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize};
use casement_core::output::{EdidFacts, EdidSerial};
use casement_core::time::Mono;
use casement_core::window::FrameHint;
use casement_core::{OutputKey, OutputName, WindowId};
use casement_layout::{Arrangement, ConfigureNeed, OverviewSlot, SlotKind, WindowTarget};
use casement_motion::MotionBook;
use casement_motion::{CommittedSizes, FrameScene, PlacedWindow, WindowLook, WindowLooks, sample};
use casement_protocols::vocab::{Emphasis, FrameMode};
use std::time::Duration;

pub fn at(ms: u64) -> Mono {
    Mono(Duration::from_millis(ms))
}

pub fn at_us(us: u64) -> Mono {
    Mono(Duration::from_micros(us))
}

pub fn output(name: &str) -> OutputKey {
    let edid = EdidFacts {
        make: String::new(),
        model: String::new(),
        serial: EdidSerial::Unknown,
    };
    OutputKey::of(&edid, &OutputName::parse(name).expect("name"))
}

pub fn rect(x: i32, y: i32, w: u32, h: u32) -> LogicalRect {
    LogicalRect::new(LogicalPoint { x, y }, LogicalSize { w, h })
}

/// An arrangement of `(window, output, rect)` rows in stacking order, bottom first.
pub fn arranged(windows: &[(u64, &str, LogicalRect)]) -> Arrangement {
    Arrangement {
        windows: windows
            .iter()
            .map(|(n, on, rect)| WindowTarget {
                window: WindowId(*n),
                output: output(on),
                rect: *rect,
                size: ConfigureNeed::Keep,
            })
            .collect(),
        ..Arrangement::default()
    }
}

/// `arrangement` with these windows sent to these overview slots.
pub fn with_slots(mut arrangement: Arrangement, slots: &[(u64, LogicalRect)]) -> Arrangement {
    arrangement.overview = slots
        .iter()
        .map(|(n, rect)| OverviewSlot {
            window: WindowId(*n),
            kind: SlotKind::Live,
            rect: *rect,
            icon: *rect,
        })
        .collect();
    arrangement
}

pub fn committed(windows: &[(u64, u32, u32)]) -> CommittedSizes {
    CommittedSizes(
        windows
            .iter()
            .map(|(n, w, h)| (WindowId(*n), LogicalSize { w: *w, h: *h }))
            .collect(),
    )
}

pub fn look(emphasis: Emphasis, mode: FrameMode, corner_radius: u32) -> WindowLook {
    WindowLook {
        emphasis,
        frame: FrameHint {
            mode,
            corner_radius,
        },
    }
}

/// Inactive windows with a compositor shadow and a 10 px corner.
pub fn looks(windows: &[u64]) -> WindowLooks {
    let look = look(Emphasis::Inactive, FrameMode::CompositorShadow, 10);
    WindowLooks(windows.iter().map(|n| (WindowId(*n), look)).collect())
}

pub fn placed(frame: &FrameScene, n: u64) -> Option<PlacedWindow> {
    frame
        .outputs
        .iter()
        .flat_map(|o| o.windows.iter())
        .find(|w| w.window == WindowId(n))
        .copied()
}

/// Window `n`'s frame position at `t`, in sub-pixels: (x, y, w, h, opacity permille).
pub fn drawn(book: &MotionBook, n: u64, t: Mono) -> Option<[i64; 5]> {
    let (frame, _) = sample(book, &CommittedSizes::default(), &looks(&[n]), t);
    placed(&frame, n).map(|w| {
        [
            i64::from(w.at.x.0),
            i64::from(w.at.y.0),
            i64::from(w.size.w.0),
            i64::from(w.size.h.0),
            i64::from(w.opacity.0),
        ]
    })
}

pub mod mc_entry;
