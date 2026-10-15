//! The pointing devices the seat has, and what each was last configured with: the pure side of
//! applying a changed setting to a device that is already there (settings take effect at once,
//! as in System Settings on the Mac). [`PresentDevices::changes`] is the diff from what was
//! applied to what the settings say now; the loop carries it out through the
//! [`InputDevices`](super::InputDevices) seam.

use super::config::device_settings;
use super::devices::DeviceSettings;
use casement_core::DeviceId;
use casement_core::input::DeviceClass;
use casement_core::settings::PointerSettings;
use std::collections::BTreeMap;

/// One device the seat has.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Present {
    class: DeviceClass,
    /// What it was last configured with; none while it never took a configuration.
    applied: Option<DeviceSettings>,
}

/// The seat's devices, by id.
///
/// ```
/// use casement_backend::input::PresentDevices;
/// use casement_core::DeviceId;
/// use casement_core::input::DeviceClass;
/// use casement_core::settings::PointerSettings;
///
/// let mut present = PresentDevices::default();
/// present.appeared(DeviceId(7), DeviceClass::Pointer);
/// assert_eq!(present.changes(&PointerSettings::defaults()).len(), 1);
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PresentDevices {
    devices: BTreeMap<DeviceId, Present>,
}

impl PresentDevices {
    /// `device` appeared.
    pub fn appeared(&mut self, device: DeviceId, class: DeviceClass) {
        self.devices.insert(
            device,
            Present {
                class,
                applied: None,
            },
        );
    }

    /// `device` went away.
    pub fn gone(&mut self, device: DeviceId) {
        self.devices.remove(&device);
    }

    /// `device` took `settings`.
    pub fn applied(&mut self, device: DeviceId, settings: DeviceSettings) {
        if let Some(present) = self.devices.get_mut(&device) {
            present.applied = Some(settings);
        }
    }

    /// What each pointing device (a mouse, a trackball or a touchpad) must be configured with to
    /// match `pointer`: the devices whose last configuration differs from the one `pointer`
    /// gives their class, or that took none. A keyboard, a touchscreen or a switch has none.
    pub fn changes(&self, pointer: &PointerSettings) -> Vec<(DeviceId, DeviceSettings)> {
        self.devices
            .iter()
            .filter(|(_, present)| {
                matches!(present.class, DeviceClass::Pointer | DeviceClass::Touchpad)
            })
            .filter_map(|(id, present)| {
                let wanted = device_settings(pointer, present.class);
                (present.applied != Some(wanted)).then_some((*id, wanted))
            })
            .collect()
    }
}
