//! Pointing-device settings (`pointer.*`, `touchpad.*`): what the libinput seam applies to
//! each device (P1-I1). The stored keys (`pointer.*`, `touchpad.*`) and their conversion are in
//! `casement/src/config/`.

use super::Enabled;
use ds_core::vocab::Fraction;
use ds_core::word::Word;
use serde::{Deserialize, Serialize};

/// The pointer acceleration curve.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
#[non_exhaustive]
pub enum AccelProfile {
    /// Constant: the device moves the pointer in proportion.
    Flat,
    /// Speeds up with the device's speed.
    Adaptive,
}

/// How one kind of pointing device behaves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct PointerProfile {
    /// The acceleration curve.
    pub accel: AccelProfile,
    /// Scroll content with the fingers (the Mac's default for every pointer, mice included).
    pub natural_scroll: Enabled,
    /// Tap to click.
    pub tap: Enabled,
    /// How far a scroll step goes, in thousandths of the device's own step.
    pub scroll_factor: Fraction,
    /// Ignore the touchpad while typing.
    pub disable_while_typing: Enabled,
}

impl PointerProfile {
    /// A profile.
    pub fn new(
        accel: AccelProfile,
        natural_scroll: Enabled,
        tap: Enabled,
        scroll_factor: Fraction,
        disable_while_typing: Enabled,
    ) -> PointerProfile {
        PointerProfile {
            accel,
            natural_scroll,
            tap,
            scroll_factor,
            disable_while_typing,
        }
    }
}

/// `pointer.*` and `touchpad.*`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct PointerSettings {
    /// Mice and trackballs.
    pub mouse: PointerProfile,
    /// Touchpads.
    pub touchpad: PointerProfile,
}

impl PointerSettings {
    /// Settings with `mouse` for mice and trackballs and `touchpad` for touchpads.
    pub fn new(mouse: PointerProfile, touchpad: PointerProfile) -> PointerSettings {
        PointerSettings { mouse, touchpad }
    }

    /// The Mac's defaults: adaptive acceleration and natural scrolling on every pointer,
    /// tap-to-click and disable-while-typing on the touchpad only, scroll at the device's own
    /// step.
    pub fn defaults() -> PointerSettings {
        let step = Fraction(1000);
        PointerSettings {
            mouse: PointerProfile::new(
                AccelProfile::Adaptive,
                Enabled::On,
                Enabled::Off,
                step,
                Enabled::Off,
            ),
            touchpad: PointerProfile::new(
                AccelProfile::Adaptive,
                Enabled::On,
                Enabled::On,
                step,
                Enabled::On,
            ),
        }
    }
}
