//! What the next frame will cost, from the last few.

use std::time::Duration;

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
        let _ = cost;
        todo!("P1-D3: RenderHistory::with")
    }

    /// The costs, oldest first.
    pub fn costs(&self) -> &[Duration] {
        &self.costs
    }
}

/// The cost to plan the next frame for. An empty history predicts a conservative default.
pub fn predict(history: &RenderHistory) -> Duration {
    let _ = history;
    todo!("P1-D3: predict")
}
