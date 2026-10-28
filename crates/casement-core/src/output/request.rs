//! A client's request to change an output (`zwlr_output_manager_v1`, `zwlr_output_power_v1`):
//! what it may change, and the pure check that turns the request into a configuration.

use super::{Mode, OutputConfig, OutputFacts, OutputPower};
use crate::geometry::LogicalPoint;
use crate::ids::OutputKey;
use crate::model::OutputBook;
use casement_protocols::vocab::Transform;
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
    /// The position is further out than any arrangement of displays reaches.
    #[error("that position is out of range")]
    PositionOutOfRange(LogicalPoint),
    /// The change would leave every display dark.
    #[error("at least one display must stay lit")]
    NoneLeftLit,
}

/// The smallest scale outputs take, in 120ths (a quarter).
const MIN_SCALE: u32 = 30;

/// The largest scale outputs take, in 120ths (8x).
const MAX_SCALE: u32 = 960;

/// The furthest an output's position may lie from the origin on either axis, in logical pixels:
/// far past any real arrangement (a dozen 8K displays side by side), near enough that a
/// request cannot put an output where the pointer never goes.
const MAX_POSITION: i32 = 100_000;

/// The configuration `change` makes of `current` on the display `facts` describes, or why not.
/// A part the change leaves out keeps `current`'s value; a new mode must be one the display
/// offers.
///
/// ```
/// use casement_core::output::{OutputChangeSet, OutputConfig, OutputPower, resolve_change};
///
/// # fn demo(facts: &casement_core::output::OutputFacts, current: &OutputConfig) {
/// let dark = OutputChangeSet::none().with_power(OutputPower::Off);
/// let resolved = resolve_change(facts, current, &dark).expect("a power change is always fine");
/// assert_eq!(resolved.power, OutputPower::Off);
/// # }
/// ```
pub fn resolve_change(
    facts: &OutputFacts,
    current: &OutputConfig,
    change: &OutputChangeSet,
) -> Result<OutputConfig, OutputRefusal> {
    let mode = match change.mode {
        Some(wanted) if facts.modes.contains(&wanted) => wanted,
        Some(wanted) => return Err(OutputRefusal::ModeNotOffered(wanted)),
        None => current.mode,
    };
    let scale = match change.scale {
        Some(wanted) if (MIN_SCALE..=MAX_SCALE).contains(&wanted.numerator()) => wanted,
        Some(wanted) => return Err(OutputRefusal::ScaleOutOfRange(wanted)),
        None => current.scale,
    };
    let position = change.position.unwrap_or(current.position);
    if position.x.unsigned_abs() > MAX_POSITION.unsigned_abs()
        || position.y.unsigned_abs() > MAX_POSITION.unsigned_abs()
    {
        return Err(OutputRefusal::PositionOutOfRange(position));
    }
    Ok(OutputConfig {
        mode,
        scale,
        transform: change.transform.unwrap_or(current.transform),
        position,
        power: change.power.unwrap_or(current.power),
    })
}

/// Whether some display is still lit once `changes` are made to `outputs`. A change set that
/// turns a lit display dark and leaves none lit is refused; a state that is already dark is not
/// made worse by a change that keeps it so (CASC-09).
///
/// ```
/// use casement_core::OutputBook;
/// use casement_core::output::keeps_one_lit;
///
/// assert!(keeps_one_lit(&OutputBook::default(), &[]).is_ok());
/// ```
pub fn keeps_one_lit(
    outputs: &OutputBook,
    changes: &[(OutputKey, OutputChangeSet)],
) -> Result<(), OutputRefusal> {
    let power_after = |key: &OutputKey, now: OutputPower| {
        changes
            .iter()
            .rev()
            .find(|(changed, _)| changed == key)
            .and_then(|(_, change)| change.power)
            .unwrap_or(now)
    };
    let lit_after = outputs
        .0
        .iter()
        .any(|(key, state)| power_after(key, state.config.power) == OutputPower::On);
    let darkened = outputs.0.iter().any(|(key, state)| {
        state.config.power == OutputPower::On
            && power_after(key, state.config.power) == OutputPower::Off
    });
    match (lit_after, darkened) {
        (false, true) => Err(OutputRefusal::NoneLeftLit),
        (true, _) | (false, false) => Ok(()),
    }
}
