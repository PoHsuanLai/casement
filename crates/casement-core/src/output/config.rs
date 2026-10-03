//! How an output is configured, and the stored configuration of every display seen.

use super::Mode;
use crate::geometry::{LogicalPoint, Transform};
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
