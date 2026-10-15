//! Accessibility settings (`accessibility.*`): Reduce transparency (System Settings,
//! Accessibility, Display). The stored key and its conversion are in `casement/src/config/`.

use super::Enabled;
use ds_core::word::Word;
use serde::{Deserialize, Serialize};

/// How translucent the surfaces casement composes are: what a frame is built with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum Transparency {
    /// A surface that asked for blur behind it gets the blur.
    #[default]
    Full,
    /// No blur anywhere: a surface that asked for one gets a mostly opaque stand-in.
    Reduced,
}

/// `accessibility.*`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct AccessibilitySettings {
    /// `accessibility.reduce_transparency`: blur off everywhere, a solid stand-in instead.
    pub reduce_transparency: Enabled,
}

impl AccessibilitySettings {
    /// Settings with these fields.
    pub fn new(reduce_transparency: Enabled) -> AccessibilitySettings {
        AccessibilitySettings {
            reduce_transparency,
        }
    }

    /// The Mac's default: transparency is not reduced.
    ///
    /// ```
    /// use casement_core::settings::{AccessibilitySettings, Transparency};
    ///
    /// assert_eq!(AccessibilitySettings::defaults().transparency(), Transparency::Full);
    /// ```
    pub fn defaults() -> AccessibilitySettings {
        AccessibilitySettings::new(Enabled::Off)
    }

    /// What a frame is built with under these settings.
    ///
    /// ```
    /// use casement_core::settings::{AccessibilitySettings, Enabled, Transparency};
    ///
    /// let reduced = AccessibilitySettings::new(Enabled::On);
    /// assert_eq!(reduced.transparency(), Transparency::Reduced);
    /// ```
    pub fn transparency(&self) -> Transparency {
        match self.reduce_transparency {
            Enabled::On => Transparency::Reduced,
            Enabled::Off => Transparency::Full,
        }
    }
}
