//! A client's request to change an output (`zwlr_output_manager_v1`, `zwlr_output_power_v1`):
//! what it may change, and the pure check that turns the request into a configuration.

use super::{Mode, OutputConfig, OutputFacts, OutputPower};
use crate::geometry::{LogicalPoint, Transform};
use crate::ids::OutputKey;
use ds_core::geometry::scale::Scale;

/// The parts of an output's configuration a request changes; `None` leaves a part as it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct OutputChangeSet {
    /// A new mode, which the display must offer.
    pub mode: Option<Mode>,
    /// A new scale.
    pub scale: Option<Scale>,
    /// A new transform.
    pub transform: Option<Transform>,
    /// A new position in the global logical space.
    pub position: Option<LogicalPoint>,
    /// Lit or dark.
    pub power: Option<OutputPower>,
}

impl OutputChangeSet {
    /// A request that changes nothing.
    pub fn none() -> OutputChangeSet {
        OutputChangeSet {
            mode: None,
            scale: None,
            transform: None,
            position: None,
            power: None,
        }
    }

    /// The same, also setting the power.
    #[must_use]
    pub fn with_power(self, power: OutputPower) -> OutputChangeSet {
        OutputChangeSet {
            power: Some(power),
            ..self
        }
    }
}

/// Why a request was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum OutputRefusal {
    /// The output is not plugged.
    #[error("no such output")]
    UnknownOutput(OutputKey),
    /// The display does not offer the mode.
    #[error("the display does not offer that mode")]
    ModeNotOffered(Mode),
    /// The scale is outside what outputs take.
    #[error("that scale is not allowed")]
    ScaleOutOfRange(Scale),
}

/// The configuration `change` makes of `current` on the display `facts` describes, or why not.
pub fn resolve_change(
    facts: &OutputFacts,
    current: &OutputConfig,
    change: &OutputChangeSet,
) -> Result<OutputConfig, OutputRefusal> {
    let _ = (facts, current, change);
    todo!("P1-M1: resolve_change")
}
