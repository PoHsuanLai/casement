//! Sampling: the frame at an instant.

use crate::book::{Motion, MotionKey};
use crate::pose::Pose;
use crate::quantise::quantise;
use crate::window_motion::WindowLegs;
use crate::{
    CornerRadius, Flight, FrameScene, MotionBook, OutputFrame, PlacedWindow, Settled, ShadowLevel,
    StripOffset,
};
use casement_core::geometry::{LogicalSize, SubPoint, SubPx, SubSize};
use casement_core::time::Mono;
use casement_core::window::FrameHint;
use casement_core::{OutputKey, WindowId};
use casement_protocols::vocab::{Emphasis, FrameMode};
use ds_core::vocab::Fraction;
use std::collections::BTreeMap;

/// Each window's committed buffer size: until a client commits its new size, the frame scales
/// the old buffer over the moving rectangle. A window with no entry is taken to have committed
/// its target size.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CommittedSizes(pub BTreeMap<WindowId, LogicalSize>);

/// How each window is drawn apart from where: its emphasis and its frame (the model's, read
/// at sampling). A window with no entry is inactive with a client-drawn frame.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WindowLooks(pub BTreeMap<WindowId, WindowLook>);

/// One window's look.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowLook {
    /// Key, main or inactive.
    pub emphasis: Emphasis,
    /// Who draws its frame and its corner radius.
    pub frame: FrameHint,
}

/// The frame at `now`: every window's quantised position, size and opacity grouped by output
/// (in key order, windows bottom of the stack first) with each output's strip offset, and
/// whether anything still moves.
pub fn sample(
    book: &MotionBook,
    committed: &CommittedSizes,
    looks: &WindowLooks,
    now: Mono,
) -> (FrameScene, Settled) {
    let mut rows: Vec<(&OutputKey, u32, PlacedWindow)> = Vec::new();
    let mut frames: BTreeMap<&OutputKey, OutputFrame> = BTreeMap::new();
    for (key, motion) in &book.legs {
        match (key, motion) {
            (MotionKey::Window(id), Motion::Window(legs)) if !motion.is_gone(now) => {
                rows.push((
                    &legs.output,
                    legs.stack,
                    placed(*id, legs, committed, looks, now),
                ));
            }
            (MotionKey::SpaceStrip(output), Motion::Slide(leg)) => {
                frame_of(&mut frames, output).strip =
                    StripOffset(quantise(leg.state(now).position));
            }
            _ => {}
        }
    }
    rows.sort_by(|a, b| (a.0, a.1, a.2.window).cmp(&(b.0, b.1, b.2.window)));
    for (output, _, window) in rows {
        frame_of(&mut frames, output).windows.push(window);
    }
    let settled = match book.is_moving(now) {
        true => Settled::Moving,
        false => Settled::Rest,
    };
    let outputs = frames.into_values().collect();
    (FrameScene { outputs }, settled)
}

/// `output`'s frame in `frames`, empty if it has none yet.
fn frame_of<'b, 'f>(
    frames: &'f mut BTreeMap<&'b OutputKey, OutputFrame>,
    output: &'b OutputKey,
) -> &'f mut OutputFrame {
    frames.entry(output).or_insert_with(|| OutputFrame {
        output: output.clone(),
        strip: StripOffset(SubPx(0)),
        windows: Vec::new(),
    })
}

/// One window sampled and quantised.
fn placed(
    id: WindowId,
    legs: &WindowLegs,
    committed: &CommittedSizes,
    looks: &WindowLooks,
    now: Mono,
) -> PlacedWindow {
    let pose = legs.pose(now);
    let target = legs.target();
    let look = looks.0.get(&id).copied();
    let emphasis = look.map_or(Emphasis::Inactive, |look| look.emphasis);
    let frame = look.map(|look| look.frame);
    PlacedWindow {
        window: id,
        committed: committed.0.get(&id).copied().unwrap_or(LogicalSize {
            w: whole(target.w),
            h: whole(target.h),
        }),
        at: SubPoint {
            x: quantise(pose.x),
            y: quantise(pose.y),
        },
        size: SubSize {
            w: quantise(pose.w.max(0.0)),
            h: quantise(pose.h.max(0.0)),
        },
        opacity: permille(pose.opacity),
        corner: CornerRadius(quantise(f64::from(frame.map_or(0, |f| f.corner_radius)))),
        shadow: shadow(frame.map(|f| f.mode), emphasis),
        emphasis,
        flight: flight_of(legs, pose, now),
    }
}

/// The flight a window is on at `now`: only while it moves, and how far along its pose is.
fn flight_of(legs: &WindowLegs, pose: Pose, now: Mono) -> Option<Flight> {
    let ends = legs.flight.filter(|_| legs.is_moving(now))?;
    Some(Flight::new(
        ends.window,
        ends.tile,
        permille(ends.progress(pose)),
    ))
}

/// The shadow a window casts: none when its client draws its own frame, the deeper one for the
/// key window.
fn shadow(mode: Option<FrameMode>, emphasis: Emphasis) -> ShadowLevel {
    match (mode, emphasis) {
        (None | Some(FrameMode::Client), _) => ShadowLevel::None,
        (Some(FrameMode::CompositorShadow), Emphasis::Key) => ShadowLevel::Key,
        (Some(FrameMode::CompositorShadow), _) => ShadowLevel::Inactive,
    }
}

/// An opacity 0 to 1 in thousandths, held to that range.
fn permille(opacity: f64) -> Fraction {
    Fraction((opacity * 1000.0).round().clamp(0.0, 1000.0) as u16)
}

/// `value` as whole logical pixels, never negative.
fn whole(value: f64) -> u32 {
    value.round().max(0.0) as u32
}
