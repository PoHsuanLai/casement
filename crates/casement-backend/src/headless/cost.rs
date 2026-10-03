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
    /// Frame n takes the n-th duration (cycling; an empty script costs nothing).
    Script(Vec<Duration>),
}

impl RenderCostModel {
    /// The cost of frame `n` when it is not measured (`None` for [`RenderCostModel::Measured`]).
    pub fn cost_of(&self, n: u64) -> Option<Duration> {
        match self {
            RenderCostModel::Measured => None,
            RenderCostModel::Fixed(cost) => Some(*cost),
            RenderCostModel::Script(script) => {
                let len = u64::try_from(script.len()).unwrap_or(u64::MAX).max(1);
                let at = usize::try_from(n % len).unwrap_or(0);
                Some(script.get(at).copied().unwrap_or(Duration::ZERO))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    #[test]
    fn measured_has_no_modelled_cost() {
        assert_eq!(RenderCostModel::Measured.cost_of(0), None);
    }

    #[test]
    fn fixed_costs_the_same_every_frame() {
        let model = RenderCostModel::Fixed(ms(4));
        assert_eq!(model.cost_of(0), Some(ms(4)));
        assert_eq!(model.cost_of(u64::MAX), Some(ms(4)));
    }

    #[test]
    fn a_script_cycles() {
        let model = RenderCostModel::Script(vec![ms(1), ms(20), ms(3)]);
        let costs: Vec<_> = (0..7).map(|n| model.cost_of(n)).collect();
        let want = [1, 20, 3, 1, 20, 3, 1].map(|n| Some(ms(n)));
        assert_eq!(costs, want);
    }

    #[test]
    fn an_empty_script_costs_nothing() {
        assert_eq!(RenderCostModel::Script(Vec::new()).cost_of(5), Some(ms(0)));
    }
}
