//! Output requests from clients: a changed configuration (output management, output power)
//! and gamma control.

use super::Model;
use super::moved::Moved;
use crate::effect::{CoreNote, Effect};
use crate::ids::OutputKey;
use crate::output::{GammaRamp, OutputChangeSet, OutputRefusal, keeps_one_lit, resolve_change};

impl Model {
    /// A client asked to change `output`: checked against its facts with
    /// `output::resolve_change`, then `Effect::ApplyOutput` with the new configuration (and an
    /// arrangement redraw), or a note when it was refused. A request that changes nothing
    /// moves nothing, and one that would leave every display dark is refused.
    pub(super) fn request_output(self, output: OutputKey, change: OutputChangeSet) -> Moved {
        let Some(plugged) = self.outputs.0.get(&output).cloned() else {
            let refusal = OutputRefusal::UnknownOutput(output);
            return Moved::of(self).with(Effect::Note(CoreNote::RefusedOutput(refusal)));
        };
        let resolved =
            resolve_change(&plugged.facts, &plugged.config, &change).and_then(|config| {
                keeps_one_lit(&self.outputs, &[(output.clone(), change)]).map(|()| config)
            });
        match resolved {
            Err(refusal) => Moved::of(self).with(Effect::Note(CoreNote::RefusedOutput(refusal))),
            Ok(config) if config == plugged.config => Moved::of(self),
            Ok(config) => Moved::of(self)
                .with(Effect::ApplyOutput {
                    output: output.clone(),
                    config,
                })
                .then(|model| model.configure_output(&output, config)),
        }
    }

    /// A client set `output`'s gamma: `Effect::SetGamma` for a plugged output, nothing for
    /// one that is gone.
    pub(super) fn set_gamma(self, output: OutputKey, ramp: GammaRamp) -> Moved {
        match self.outputs.0.contains_key(&output) {
            true => Moved::of(self.night_client_ramp(&output, &ramp))
                .with(Effect::SetGamma(output, ramp))
                .then(Model::night_drive),
            false => Moved::of(self),
        }
    }
}
