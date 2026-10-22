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
    /// Its whole rectangle, in global logical coordinates (what a Split View covers).
    pub full: LogicalRect,
    /// What a full-screen window covers: all of `full`, or (the shell keeps the bar up in full
    /// screen) `full` less the bar's exclusive zone. Inside `full`.
    pub covered: LogicalRect,
    /// Its area left after layer exclusive zones, in global logical coordinates.
    pub usable: LogicalRect,
    /// Its scale.
    pub scale: Scale,
    /// What it shows.
    pub mode: SceneMode,
    /// Its Spaces, in row order.
    pub spaces: Vec<SpaceSpec>,
}
