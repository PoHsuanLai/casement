//! What a virtual input device is.

use casement_core::input::{DeviceClass, Origin};
use casement_core::settings::XkbNames;

/// A virtual device's description.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceSpec {
    /// What kind of device.
    pub class: DeviceClass,
    /// The origin every event from it carries (set here, never changed downstream, I8).
    pub origin: Origin,
    /// Its keymap (keyboards).
    pub layout: XkbNames,
    /// Its name, as a device list shows it.
    pub name: DeviceName,
}

/// A device's name.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DeviceName(pub String);
