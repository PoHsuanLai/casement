//! What rendering a frame costs, in session time: measured on real backends, fixed or
//! scripted in tests so frame-miss counts are exact.

use std::time::Duration;

/// How long a frame's rendering takes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderCostModel {
    /// The time it really took (the wall clock, read only in `casement/src/time/`).
    Measured,
    /// Every frame takes this long.
    Fixed(Duration),
    /// Frame n takes the n-th duration (cycling).
    Script(Vec<Duration>),
}

impl RenderCostModel {
    /// The cost of frame `n` when it is not measured (`None` for [`RenderCostModel::Measured`]).
    pub fn cost_of(&self, n: u64) -> Option<Duration> {
        let _ = n;
        todo!("P0-B-headless: RenderCostModel::cost_of")
    }
}
