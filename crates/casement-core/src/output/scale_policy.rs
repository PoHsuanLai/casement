//! The scale a display gets the first time it is seen (12 §4: the policy moves from sill's
//! display service into the compositor, so a 4K output is right before any client draws).
//! The `WholeLikeMac` rows are sill's `PreferWhole` table (`displays/scale.rs`), in 120ths.

use super::density::{MilliPpi, density};
use super::{ModeWish, OutputFacts, pick_mode};
use ds_core::geometry::scale::Scale;
use serde::{Deserialize, Serialize};

/// The Mac's 27-inch density (2560 x 1440 points on 27 inches), in pixels per inch.
const TARGET_PPI: u64 = 109;
/// The ideal scale (hundredths) from which a whole scale is preferred: 1.5.
const WHOLE_FROM: u64 = 150;
/// The narrowest logical width a whole scale may leave: the smallest common laptop panel.
const MIN_LOGICAL_WIDTH: u32 = 1280;
/// The largest whole scale chosen: 3x on a 15.6-inch 4K laptop leaves 1280 points, tighter than
/// any Mac laptop's default, so a denser panel stays at 2x.
const MAX_WHOLE: u64 = 2;
/// How close (hundredths) to a whole scale the ideal must be to take it.
const WHOLE_PULL: u64 = 15;
/// The steps a first scale may land on, in hundredths: 1, 1.25, 1.5, 1.75, 2, 2.5, 3.
const STEPS: [u64; 7] = [100, 125, 150, 175, 200, 250, 300];
/// A panel at least this many pixels wide, with no usable size, is a 4K TV or projector,
/// viewed from further than its size would say: 2x.
const WIDE_PANEL: u32 = 3840;

/// How a display's first scale is chosen (`outputs.first_sight_scale`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum ScalePolicy {
    /// Aim at the Mac's 27-inch density (109 points per inch) and prefer a whole scale where
    /// the ideal is at least 1.5 and the display stays at least 1280 points wide, as macOS does
    /// for a sharp external display; otherwise the nearest quarter step. With no usable
    /// physical size: 2 for a panel at least 3840 pixels wide, else 1.
    WholeLikeMac,
    /// Always this scale.
    #[serde(with = "crate::geometry::scale_serde")]
    Fixed(Scale),
}

/// The scale for a display seen for the first time.
pub fn first_sight_scale(facts: &OutputFacts, policy: &ScalePolicy) -> Scale {
    match policy {
        ScalePolicy::Fixed(scale) => *scale,
        ScalePolicy::WholeLikeMac => {
            let pixels = pick_mode(facts, &ModeWish::Preferred).size;
            let hundredths = match density(pixels, facts.physical) {
                Some(ppi) => whole_like_mac(ideal(ppi), pixels.w),
                None if pixels.w >= WIDE_PANEL => 200,
                None => 100,
            };
            from_hundredths(hundredths)
        }
    }
}

/// The scale (hundredths) that would show `ppi` at the Mac's density, rounded half up.
fn ideal(ppi: MilliPpi) -> u64 {
    (ppi.0 * 100 + TARGET_PPI * 500) / (TARGET_PPI * 1000)
}

/// Below 1.5, the nearest step; from it, a whole scale (at most 2) when the display stays wide
/// enough, else the nearest step.
fn whole_like_mac(ideal: u64, width: u32) -> u64 {
    let whole = nearest_whole(ideal).min(MAX_WHOLE);
    let wide_enough = u64::from(width) / whole >= u64::from(MIN_LOGICAL_WIDTH);
    if ideal >= WHOLE_FROM && wide_enough {
        whole * 100
    } else {
        snap(ideal)
    }
}

/// `ideal` rounded half up to a whole scale, at least 1 and at most 3.
fn nearest_whole(ideal: u64) -> u64 {
    ((ideal + 50) / 100).clamp(1, 3)
}

/// The whole scale when `ideal` is within [`WHOLE_PULL`] of one, else the nearest step (the
/// lower of two equally near).
fn snap(ideal: u64) -> u64 {
    let whole = nearest_whole(ideal) * 100;
    if ideal.abs_diff(whole) <= WHOLE_PULL {
        return whole;
    }
    STEPS
        .iter()
        .copied()
        .min_by_key(|step| step.abs_diff(ideal))
        .unwrap_or(100)
}

/// `hundredths` of a scale in 120ths (exact for every step).
fn from_hundredths(hundredths: u64) -> Scale {
    let percent = u16::try_from(hundredths).unwrap_or(u16::MAX);
    Scale::from_percent(percent)
}
