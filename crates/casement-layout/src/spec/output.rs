//! One output's part of the scene.

use super::{SceneMode, SpaceSpec};
use casement_core::OutputKey;
use casement_core::geometry::LogicalRect;
use ds_core::geometry::scale::Scale;

/// One output: where windows may go, at what scale, in which mode, with which Spaces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputSpec {
    /// The output.
    pub output: OutputKey,
    /// Its whole rectangle, in global logical coordinates (what a full-screen window covers).
    pub full: LogicalRect,
    /// Its area left after layer exclusive zones, in global logical coordinates.
    pub usable: LogicalRect,
    /// Its scale.
    pub scale: Scale,
    /// What it shows.
    pub mode: SceneMode,
    /// Its Spaces, in row order.
    pub spaces: Vec<SpaceSpec>,
}
