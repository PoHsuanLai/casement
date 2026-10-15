//! The whole scene.

use super::{OutputSpec, OverviewKind, SceneMode};

/// Every output to arrange.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneSpec {
    /// The outputs, in the model's order.
    pub outputs: Vec<OutputSpec>,
}

impl SceneSpec {
    /// The same scene with every output that shows a desktop in Mission Control over it: the
    /// arrangement then has the spread's slots for the shown Spaces' windows. An output already
    /// in an overview, or in Show Desktop, is left as it is.
    ///
    /// ```
    /// use casement_layout::SceneSpec;
    ///
    /// let scene = SceneSpec { outputs: Vec::new() };
    /// assert_eq!(scene.clone().in_mission_control(), scene);
    /// ```
    pub fn in_mission_control(self) -> SceneSpec {
        let outputs = self
            .outputs
            .into_iter()
            .map(|output| match output.mode {
                SceneMode::Desktop { shown } => OutputSpec {
                    mode: SceneMode::Overview {
                        shown,
                        kind: OverviewKind::MissionControl,
                    },
                    ..output
                },
                _ => output,
            })
            .collect();
        SceneSpec { outputs }
    }
}
