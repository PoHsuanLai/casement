//! The real libinput device configuration (feature `libinput`): `InputDevices` over the input
//! crate, a thin adapter (B7). P1-Z declares it; P1-I1 fills it.
//!
//! The adapter owns the libinput context for the seat and the devices it has been told about
//! ([`RealInputDevices::track`]); the loop that dispatches libinput's events tells it as devices
//! are added and removed. Scroll factor is not a libinput setting (libinput scrolls by the
//! device's own step): the compositor scales the axis event, so it is never reported as
//! unsupported here.

use super::devices::{Applied, DeviceSetting, DeviceSettings, InputDevices, InputError};
use casement_core::DeviceId;
use casement_core::settings::{AccelProfile, Enabled};
use smithay::reexports::input::{
    self as libinput, DeviceConfigError, DeviceConfigResult, Libinput, LibinputInterface,
};
use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::os::fd::OwnedFd;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

/// Opens device nodes directly. The seat's own descriptors (logind's `TakeDevice`) replace this
/// when the tty backend hands its session over (P1-D1); until then libinput needs read access
/// to the nodes.
#[derive(Debug)]
struct DirectOpen;

impl LibinputInterface for DirectOpen {
    fn open_restricted(&mut self, path: &Path, flags: i32) -> Result<OwnedFd, i32> {
        OpenOptions::new()
            .custom_flags(flags)
            .read(true)
            .write(true)
            .open(path)
            .map(OwnedFd::from)
            .map_err(|error| error.raw_os_error().unwrap_or(libc_eio()))
    }

    fn close_restricted(&mut self, fd: OwnedFd) {
        drop(fd);
    }
}

/// `EIO`, for an error without an errno.
const fn libc_eio() -> i32 {
    5
}

/// libinput's devices on a seat.
#[derive(Debug)]
pub struct RealInputDevices {
    /// The seat.
    pub seat: String,
    context: Libinput,
    devices: BTreeMap<DeviceId, libinput::Device>,
}

impl RealInputDevices {
    /// Opens libinput on `seat`.
    pub fn open(seat: &str) -> Result<RealInputDevices, InputError> {
        let mut context = Libinput::new_with_udev(DirectOpen);
        context.udev_assign_seat(seat).map_err(|()| {
            InputError::Io(std::io::Error::other(format!(
                "libinput could not take seat {seat}"
            )))
        })?;
        Ok(RealInputDevices {
            seat: seat.to_owned(),
            context,
            devices: BTreeMap::new(),
        })
    }

    /// The libinput context, for the loop that dispatches its events.
    pub fn context(&mut self) -> &mut Libinput {
        &mut self.context
    }

    /// Remembers `device` as `id`, so settings can be applied to it.
    pub fn track(&mut self, id: DeviceId, device: libinput::Device) {
        self.devices.insert(id, device);
    }

    /// Forgets the device `id` (it went away).
    pub fn forget(&mut self, id: DeviceId) {
        self.devices.remove(&id);
    }
}

fn on(enabled: Enabled) -> bool {
    matches!(enabled, Enabled::On)
}

/// What a device that refused a setting means for the result: unsupported and invalid both skip
/// the setting.
fn skipped(result: DeviceConfigResult, setting: DeviceSetting) -> Option<DeviceSetting> {
    match result {
        Ok(()) => None,
        Err(DeviceConfigError::Unsupported | DeviceConfigError::Invalid) => Some(setting),
    }
}

impl InputDevices for RealInputDevices {
    fn apply(&mut self, device: DeviceId, settings: &DeviceSettings) -> Applied {
        let Some(device) = self.devices.get_mut(&device) else {
            return Applied::Refused;
        };
        let profile = match settings.accel {
            AccelProfile::Flat => libinput::AccelProfile::Flat,
            _ => libinput::AccelProfile::Adaptive,
        };
        let skipped: Vec<DeviceSetting> = [
            skipped(
                device.config_accel_set_profile(profile),
                DeviceSetting::Accel,
            ),
            skipped(
                device.config_scroll_set_natural_scroll_enabled(on(settings.natural_scroll)),
                DeviceSetting::NaturalScroll,
            ),
            skipped(
                device.config_tap_set_enabled(on(settings.tap)),
                DeviceSetting::Tap,
            ),
            skipped(
                device.config_dwt_set_enabled(on(settings.disable_while_typing)),
                DeviceSetting::DisableWhileTyping,
            ),
        ]
        .into_iter()
        .flatten()
        .collect();
        if skipped.is_empty() {
            Applied::All
        } else {
            Applied::Partly(skipped)
        }
    }
}
