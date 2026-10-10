//! What the next frame will cost, from the last few.

use std::time::Duration;

/// How many recent frames the prediction reads.
const WINDOW: usize = 8;

/// What an empty history predicts: half a 60 Hz period, enough for a first frame on most
/// hardware and late enough to waste little.
const DEFAULT_COST: Duration = Duration::from_millis(8);

/// The render costs of the most recent frames, newest last.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RenderHistory {
    costs: Vec<Duration>,
}

impl RenderHistory {
    /// No frames yet.
    pub fn empty() -> RenderHistory {
        RenderHistory { costs: Vec::new() }
    }

    /// The history with one more frame of `cost`, the oldest dropped past the window the
    /// prediction reads.
    #[must_use]
    pub fn with(self, cost: Duration) -> RenderHistory {
        let mut costs = self.costs;
        costs.push(cost);
        let extra = costs.len().saturating_sub(WINDOW);
        costs.drain(..extra);
        RenderHistory { costs }
    }

    /// The costs, oldest first.
    pub fn costs(&self) -> &[Duration] {
        &self.costs
    }
}

/// The cost to plan the next frame for: the worst of the last eight frames (a missed vblank
/// costs more than rendering a little early). An empty history predicts 8 ms.
///
/// ```no_run
/// use casement_core::frame::{RenderHistory, predict};
/// use std::time::Duration;
///
/// let history = RenderHistory::empty().with(Duration::from_millis(3));
/// assert_eq!(predict(&history), Duration::from_millis(3));
/// ```
pub fn predict(history: &RenderHistory) -> Duration {
    history.costs.iter().copied().max().unwrap_or(DEFAULT_COST)
}
