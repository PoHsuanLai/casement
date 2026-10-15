//! Night light settings (`night_light.*`): the software warm-up casement drives outputs with
//! when no DDC warmth is available, and the outputs it leaves alone because they warm themselves
//! (`night_light.skip_outputs`). The stored keys and their conversion are in `casement/src/config/`.

use crate::ids::{OutputKey, OutputName};
use crate::output::{GammaRamp, Kelvin, MinuteOfDay, NightLight, Schedule};
use ds_core::word::Word;
use serde::{Deserialize, Serialize};

/// When night light is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum NightLightMode {
    /// Never.
    #[default]
    Off,
    /// All day.
    Always,
    /// From `night_light.from` to `night_light.to`.
    Scheduled,
}

/// Whether night light warms an output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputWarming {
    /// Casement drives the output's colour curve.
    Warmed,
    /// The output is listed in `night_light.skip_outputs`: casement leaves its curve alone.
    Skipped,
}

/// `night_light.*`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct NightLightSettings {
    /// `night_light.mode`.
    pub mode: NightLightMode,
    /// `night_light.from`: minute of the day warming starts.
    pub from: MinuteOfDay,
    /// `night_light.to`: minute of the day warming ends.
    pub to: MinuteOfDay,
    /// `night_light.kelvin`: how warm the screen gets.
    pub warmth: Kelvin,
    /// `night_light.fade_minutes`.
    pub fade_minutes: u16,
    /// `night_light.skip_outputs`: outputs casement does not warm, each a connector name
    /// (`DP-1`) or an [`OutputKey`]'s text (`edid:<make>/<model>/<serial>`).
    pub skip_outputs: Vec<String>,
}

impl NightLightSettings {
    /// These settings.
    pub fn new(
        mode: NightLightMode,
        (from, to): (MinuteOfDay, MinuteOfDay),
        warmth: Kelvin,
        fade_minutes: u16,
    ) -> NightLightSettings {
        NightLightSettings {
            mode,
            from,
            to,
            warmth,
            fade_minutes,
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

    /// Off; when turned on it warms 22:00 to 07:00 to 3400 K over half an hour.
    ///
    /// ```
    /// use casement_core::settings::{NightLightMode, NightLightSettings};
    ///
    /// assert_eq!(NightLightSettings::defaults().mode, NightLightMode::Off);
    /// ```
    pub fn defaults() -> NightLightSettings {
        NightLightSettings {
            mode: NightLightMode::Off,
            from: MinuteOfDay::wrapping(22 * 60),
            to: MinuteOfDay::wrapping(7 * 60),
            warmth: Kelvin::NIGHT,
            fade_minutes: 30,
            skip_outputs: Vec::new(),
        }
    }

    /// The light these settings describe.
    pub fn light(&self) -> NightLight {
        let schedule = match self.mode {
            NightLightMode::Off => Schedule::Off,
            NightLightMode::Always => Schedule::Always,
            NightLightMode::Scheduled => Schedule::Between {
                from: self.from,
                to: self.to,
            },
        };
        NightLight {
            schedule,
            warmth: self.warmth,
            fade_minutes: self.fade_minutes,
        }
    }

    /// Whether night light warms the output on connector `name` with key `key`: not when
    /// `night_light.skip_outputs` lists either (sill lists the monitors it warms itself).
    ///
    /// ```
    /// use casement_core::output::{EdidFacts, EdidSerial};
    /// use casement_core::settings::{NightLightSettings, OutputWarming};
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
    /// assert_eq!(skipping.warming_of(&name, &key), OutputWarming::Skipped);
    /// let warming = NightLightSettings::defaults();
    /// assert_eq!(warming.warming_of(&name, &key), OutputWarming::Warmed);
    /// ```
    pub fn warming_of(&self, name: &OutputName, key: &OutputKey) -> OutputWarming {
        let listed = self
            .skip_outputs
            .iter()
            .map(|entry| entry.trim())
            .any(|entry| entry == name.as_str() || entry == key.as_str());
        match listed {
            true => OutputWarming::Skipped,
            false => OutputWarming::Warmed,
        }
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
        match self.warming_of(name, key) {
            OutputWarming::Skipped => GammaRamp::Identity,
            OutputWarming::Warmed => self.light().ramp_at(now, size),
        }
    }
}
