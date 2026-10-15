//! Night light: the ramp each output is driven with. The edge reads the local minute and tells
//! the model (`Event::NightLightTick`); the model asks `Effect::SetGamma` for each output whose
//! wanted ramp is not the one it last asked for. An output a gamma-control client holds with a
//! table of its own is left to that client until it lets go, and an output listed in
//! `night_light.skip_outputs` is never warmed (`NightLightSettings::ramp_for`).

use super::Model;
use super::moved::Moved;
use crate::effect::Effect;
use crate::ids::OutputKey;
use crate::output::{GammaRamp, MinuteOfDay};
use std::collections::{BTreeMap, BTreeSet};

/// The steps per channel of the ramps night light builds; the backend resamples a table to the
/// device's own size, and a warm-up is a straight line, so nothing is lost.
const STEPS: u32 = 256;

/// What night light knows between steps.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) struct NightDrive {
    /// The local minute of the latest tick; nothing is driven before the first.
    minute: Option<MinuteOfDay>,
    /// The ramp last asked of each output (the identity for one never asked).
    asked: BTreeMap<OutputKey, GammaRamp>,
    /// Outputs a client controls with a table of its own.
    held: BTreeSet<OutputKey>,
}

impl Model {
    /// The local minute is `minute`: every output is brought to the ramp it wants now.
    pub(super) fn night_tick(self, minute: MinuteOfDay) -> Moved {
        let mut model = self;
        model.night.minute = Some(minute);
        model.night_drive()
    }

    /// Asks `Effect::SetGamma` for each plugged output that is not held and whose wanted ramp
    /// differs from the one last asked. Nothing before the first tick.
    pub(super) fn night_drive(self) -> Moved {
        let Some(minute) = self.night.minute else {
            return Moved::of(self);
        };
        let identity = GammaRamp::Identity;
        let wanted: Vec<(OutputKey, GammaRamp)> = self
            .outputs
            .0
            .iter()
            .filter(|(key, _)| !self.night.held.contains(*key))
            .map(|(key, state)| {
                let ramp = self.settings.night_light.ramp_for(
                    &state.facts.name,
                    &state.facts.key,
                    minute,
                    STEPS,
                );
                (key.clone(), ramp)
            })
            .filter(|(key, ramp)| self.night.asked.get(key).unwrap_or(&identity) != ramp)
            .collect();
        let mut model = self;
        let effects: Vec<Effect> = wanted
            .into_iter()
            .map(|(key, ramp)| {
                model.night.asked.insert(key.clone(), ramp.clone());
                Effect::SetGamma(key, ramp)
            })
            .collect();
        Moved::of(model).with_all(effects)
    }

    /// A client set `output`'s ramp (or gave it back with the identity): a table holds the
    /// output, the identity releases it, and night light takes it up again.
    pub(super) fn night_client_ramp(self, output: &OutputKey, ramp: &GammaRamp) -> Model {
        let mut model = self;
        match ramp {
            GammaRamp::Table(_) => model.night.held.insert(output.clone()),
            _ => model.night.held.remove(output),
        };
        model.night.asked.insert(output.clone(), ramp.clone());
        model
    }

    /// `output` is gone: nothing is remembered of it.
    pub(super) fn night_forget(self, output: &OutputKey) -> Model {
        let mut model = self;
        model.night.held.remove(output);
        model.night.asked.remove(output);
        model
    }
}
