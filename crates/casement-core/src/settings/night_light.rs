//! Night light settings (`night_light.*`): the software warm-up casement drives every output with
//! when no DDC warmth is available. The stored keys and their conversion are in
//! `casement/src/config/`.

use crate::output::{Kelvin, MinuteOfDay, NightLight, Schedule};
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

/// `night_light.*`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
}
