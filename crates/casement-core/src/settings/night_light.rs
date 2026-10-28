//! Night light settings (`night_light.*`): the software warm-up casement drives outputs with
//! when no DDC warmth is available, and the outputs it leaves alone because they warm themselves
//! (`night_light.skip_outputs`). The stored keys and their conversion are in `casement/src/config/`.

use crate::ids::{OutputKey, OutputName};
use crate::output::{GammaRamp, Kelvin, MinuteOfDay, NightLight, Schedule};

/// `night_light.*`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct NightLightSettings {
    /// `night_light.mode`, `from`, `to`, `kelvin` and `fade_minutes`: when the screen warms, how
    /// warm and how slowly.
    pub light: NightLight,
    /// `night_light.skip_outputs`: outputs casement does not warm, each a connector name
    /// (`DP-1`) or an [`OutputKey`]'s text (`edid:<make>/<model>/<serial>`).
    pub skip_outputs: Vec<String>,
}

impl NightLightSettings {
    /// These settings, leaving no output alone.
    pub fn new(light: NightLight) -> NightLightSettings {
        NightLightSettings {
            light,
            skip_outputs: Vec::new(),
        }
    }

    /// The same, leaving `outputs` alone.
    #[must_use]
    pub fn skipping(self, outputs: Vec<String>) -> NightLightSettings {
        NightLightSettings {
            skip_outputs: outputs,
            ..self
        }
    }

    /// Off; when turned on it warms to 3400 K over half an hour.
    ///
    /// ```
    /// use casement_core::output::Schedule;
    /// use casement_core::settings::NightLightSettings;
    ///
    /// assert_eq!(NightLightSettings::defaults().light.schedule, Schedule::Off);
    /// ```
    pub fn defaults() -> NightLightSettings {
        NightLightSettings::new(NightLight {
            schedule: Schedule::Off,
            warmth: Kelvin::NIGHT,
            fade_minutes: 30,
        })
    }

    /// Whether night light leaves the output on connector `name` with key `key` alone: when
    /// `night_light.skip_outputs` lists either (sill lists the monitors it warms itself).
    ///
    /// ```
    /// use casement_core::output::{EdidFacts, EdidSerial};
    /// use casement_core::settings::NightLightSettings;
    /// use casement_core::{OutputKey, OutputName};
    ///
    /// let name = OutputName::parse("DP-1").expect("a connector name");
    /// let edid = EdidFacts {
    ///     make: "ACME".to_owned(),
    ///     model: "M1".to_owned(),
    ///     serial: EdidSerial::Unknown,
    /// };
    /// let key = OutputKey::of(&edid, &name);
    /// let skipping = NightLightSettings::defaults().skipping(vec!["DP-1".to_owned()]);
    /// assert!(skipping.skips(&name, &key));
    /// let warming = NightLightSettings::defaults();
    /// assert!(!warming.skips(&name, &key));
    /// ```
    pub fn skips(&self, name: &OutputName, key: &OutputKey) -> bool {
        self.skip_outputs
            .iter()
            .map(|entry| entry.trim())
            .any(|entry| entry == name.as_str() || entry == key.as_str())
    }

    /// The ramp to drive the output on `name` (key `key`) with at `now`, `size` steps long: the
    /// light's, or the identity for an output that is left alone. The one place the skip list
    /// is honoured.
    pub fn ramp_for(
        &self,
        name: &OutputName,
        key: &OutputKey,
        now: MinuteOfDay,
        size: u32,
    ) -> GammaRamp {
        match self.skips(name, key) {
            true => GammaRamp::Identity,
            false => self.light.ramp_at(now, size),
        }
    }
}
