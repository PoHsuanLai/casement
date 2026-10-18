//! The libinput settings seam (P1-Z; filled by P1-I1): what each input device is configured
//! with. Real: `libinput_real.rs` (feature `libinput`); fake: `casement_testkit::fakes::
//! FakeInputDevices`, which records what was applied so a test asserts the settings at the seam.
//!
//! ```no_run
//! use casement_backend::input::{AccelProfile, DeviceSettings, InputFeed};
//! use casement_core::DeviceId;
//! use casement_core::settings::Enabled;
//! use ds_core::vocab::Fraction;
//!
//! fn natural(devices: &mut dyn InputFeed, device: DeviceId) {
//!     let settings = DeviceSettings::new(
//!         AccelProfile::Adaptive,
//!         Enabled::On,
//!         Enabled::On,
//!         Fraction(1000),
//!         Enabled::On,
//!     );
//!     let _ = devices.apply(device, &settings);
//! }
//! ```

pub use casement_core::settings::AccelProfile;

use crate::session::SessionFault;
use casement_core::DeviceId;
use casement_core::settings::Enabled;
use ds_core::vocab::Fraction;

/// What one device is configured with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct DeviceSettings {
    /// The acceleration curve.
    pub accel: AccelProfile,
    /// Scroll content with the fingers.
    pub natural_scroll: Enabled,
    /// Tap to click.
    pub tap: Enabled,
    /// How far a scroll step goes, in thousandths of the device's own step.
    pub scroll_factor: Fraction,
    /// Ignore the touchpad while typing.
    pub disable_while_typing: Enabled,
}

impl DeviceSettings {
    /// A device's settings.
    pub fn new(
        accel: AccelProfile,
        natural_scroll: Enabled,
        tap: Enabled,
        scroll_factor: Fraction,
        disable_while_typing: Enabled,
    ) -> DeviceSettings {
        DeviceSettings {
            accel,
            natural_scroll,
            tap,
            scroll_factor,
            disable_while_typing,
        }
    }
}

/// One of the settings a device may not support.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DeviceSetting {
    /// The acceleration curve.
    Accel,
    /// Natural scrolling.
    NaturalScroll,
    /// Tap to click.
    Tap,
    /// The scroll factor.
    ScrollFactor,
    /// Disable-while-typing.
    DisableWhileTyping,
}

/// What applying settings did.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Applied {
    /// Every setting took.
    All,
    /// These settings are not supported by the device and were skipped.
    Partly(Vec<DeviceSetting>),
    /// The device is not known (it went away).
    Refused,
}

/// Why the input devices could not be opened.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum InputError {
    /// The seat refused to open an input device.
    #[error("session: {0}")]
    Session(#[from] SessionFault),
    /// The input system could not start.
    #[error("input: {0}")]
    Io(#[from] std::io::Error),
}
