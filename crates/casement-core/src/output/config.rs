//! How an output is configured, and the stored configuration of every display seen.

use super::Mode;
use crate::geometry::{LogicalPoint, LogicalRect, LogicalSize, PhysicalSize, Transform};
use crate::ids::OutputKey;
use ds_core::geometry::scale::Scale;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// One output's configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputConfig {
    /// Its mode.
    pub mode: Mode,
    /// Its scale.
    #[serde(with = "crate::geometry::scale_serde")]
    pub scale: Scale,
    /// Its transform.
    pub transform: Transform,
    /// Its top-left corner in the global logical space.
    pub position: LogicalPoint,
    /// Whether it is lit.
    pub power: OutputPower,
}

impl OutputConfig {
    /// The output's whole rectangle in the global logical space: its mode (turned for a
    /// sideways transform) divided by its scale, each side rounded to the nearest logical
    /// pixel, at its position.
    pub fn logical_rect(&self) -> LogicalRect {
        let PhysicalSize { w, h } = turned(self.mode.size, self.transform);
        let n = u64::from(self.scale.numerator()).max(1);
        let denominator = u64::from(Scale::DENOMINATOR);
        let side = |pixels: u32| {
            u32::try_from((u64::from(pixels) * denominator + n / 2) / n).unwrap_or(u32::MAX)
        };
        LogicalRect::new(
            self.position,
            LogicalSize {
                w: side(w),
                h: side(h),
            },
        )
    }
}

/// `size` as scanout sees it after `transform`: the sides swap when it turns a quarter.
fn turned(size: PhysicalSize, transform: Transform) -> PhysicalSize {
    match transform {
        Transform::R90 | Transform::R270 | Transform::Flipped90 | Transform::Flipped270 => {
            PhysicalSize {
                w: size.h,
                h: size.w,
            }
        }
        Transform::Normal | Transform::R180 | Transform::Flipped | Transform::Flipped180 => size,
    }
}

/// Whether an output is lit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputPower {
    /// Lit.
    On,
    /// Dark (DPMS off).
    Off,
}

/// The configuration of every display seen, by key (`$XDG_STATE_HOME/casement/outputs.toml`).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SavedOutputs(pub BTreeMap<OutputKey, OutputConfig>);
