//! Output requests from clients: a changed configuration (output management, output power)
//! and gamma control. P1-M1 fills both.

use super::Model;
use super::moved::Moved;
use crate::ids::OutputKey;
use crate::output::{GammaRamp, OutputChangeSet};

impl Model {
    /// A client asked to change `output`: checked against its facts with
    /// `output::resolve_change`, then `Effect::ApplyOutput` with the new configuration (and an
    /// arrangement redraw), or a note when it was refused.
    pub(super) fn request_output(self, output: OutputKey, change: OutputChangeSet) -> Moved {
        let _ = (self, output, change);
        todo!("P1-M1: Model::request_output")
    }

    /// A client set `output`'s gamma: `Effect::SetGamma` for a plugged output, nothing for
    /// one that is gone.
    pub(super) fn set_gamma(self, output: OutputKey, ramp: GammaRamp) -> Moved {
        let _ = (self, output, ramp);
        todo!("P1-M1: Model::set_gamma")
    }
}
