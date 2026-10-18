//! Placing outputs in the global logical space.

use super::{ModeWish, OutputConfig, OutputFacts, OutputPower, SavedOutputs, ScalePolicy};
use super::{first_sight_scale, pick_mode};
use crate::geometry::{LogicalPoint, LogicalSize, PhysicalSize, Transform};
use crate::ids::OutputKey;
use ds_core::geometry::scale::Scale;

/// The configuration of every present output: a saved display keeps its saved configuration;
/// a new one gets its preferred mode, its first-sight scale under `policy`, no transform,
/// power on, and a place to the right of everything already placed, top-aligned. In
/// `present`'s order.
///
/// Saved displays are placed first, wherever they were; new ones follow in `present`'s order,
/// each to the right of the rightmost edge so far, at the top of the topmost output so far.
pub fn place_outputs(
    present: &[OutputFacts],
    saved: &SavedOutputs,
    policy: &ScalePolicy,
) -> Vec<(OutputKey, OutputConfig)> {
    let kept: Vec<Option<OutputConfig>> = present
        .iter()
        .map(|facts| saved.0.get(&facts.key).copied())
        .collect();
    let placed = kept
        .iter()
        .flatten()
        .fold(None, |taken, config| Some(grow(taken, config)));
    let (_, configs) = present.iter().zip(kept).fold(
        (placed, Vec::new()),
        |(taken, mut configs), (facts, kept)| {
            let config = kept.unwrap_or_else(|| fresh(facts, *policy, next_position(taken)));
            configs.push((facts.key.clone(), config));
            (Some(grow(taken, &config)), configs)
        },
    );
    configs
}

/// What a new output gets: its preferred mode at its first-sight scale, upright and lit.
fn fresh(facts: &OutputFacts, policy: ScalePolicy, position: LogicalPoint) -> OutputConfig {
    OutputConfig {
        mode: pick_mode(facts, &ModeWish::Preferred),
        scale: first_sight_scale(facts, policy),
        transform: Transform::Normal,
        position,
        power: OutputPower::On,
    }
}

/// The space the outputs placed so far take: the rightmost edge and the topmost edge.
#[derive(Debug, Clone, Copy)]
struct Taken {
    right: i64,
    top: i64,
}

/// `taken` grown by the output `config` places.
fn grow(taken: Option<Taken>, config: &OutputConfig) -> Taken {
    let size = logical_size(config);
    let right = i64::from(config.position.x) + i64::from(size.w);
    let top = i64::from(config.position.y);
    taken.map_or(Taken { right, top }, |t| Taken {
        right: t.right.max(right),
        top: t.top.min(top),
    })
}

/// Where the next new output goes: right of everything, at the top; the origin when nothing
/// is placed yet.
fn next_position(taken: Option<Taken>) -> LogicalPoint {
    let clamp = |v: i64| i32::try_from(v).unwrap_or(if v < 0 { i32::MIN } else { i32::MAX });
    taken.map_or(LogicalPoint { x: 0, y: 0 }, |t| LogicalPoint {
        x: clamp(t.right),
        y: clamp(t.top),
    })
}

/// The size an output takes in the logical space: its mode, turned by its transform, divided
/// by its scale (each side rounded to the nearest logical pixel).
fn logical_size(config: &OutputConfig) -> LogicalSize {
    let turned = turned(config.mode.size, config.transform);
    let n = i64::from(config.scale.numerator());
    let denominator = i64::from(Scale::DENOMINATOR);
    let side = |pixels: u32| {
        let logical = (i64::from(pixels) * denominator + n / 2) / n;
        u32::try_from(logical).unwrap_or(u32::MAX)
    };
    LogicalSize {
        w: side(turned.w),
        h: side(turned.h),
    }
}

/// `size` as the output shows it: width and height swap on a quarter turn.
fn turned(size: PhysicalSize, transform: Transform) -> PhysicalSize {
    match transform {
        Transform::Normal | Transform::R180 | Transform::Flipped | Transform::Flipped180 => size,
        Transform::R90 | Transform::R270 | Transform::Flipped90 | Transform::Flipped270 => {
            PhysicalSize {
                w: size.h,
                h: size.w,
            }
        }
    }
}
