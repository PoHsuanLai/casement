//! The real libinput device configuration (feature `libinput`): `InputDevices` over the input
//! crate, a thin adapter (B7). P1-Z declares it; P1-I1 fills it.

use super::devices::{Applied, DeviceSettings, InputDevices, InputError};
use casement_core::DeviceId;

/// libinput's devices on a seat.
#[derive(Debug)]
pub struct RealInputDevices {
    /// The seat.
    pub seat: String,
}

impl RealInputDevices {
    /// Opens libinput on `seat`.
    pub fn open(seat: &str) -> Result<RealInputDevices, InputError> {
        let _ = seat;
        todo!("P1-I1: RealInputDevices::open")
    }
}

impl InputDevices for RealInputDevices {
    fn apply(&mut self, device: DeviceId, settings: &DeviceSettings) -> Applied {
        let _ = (device, settings);
        todo!("P1-I1: RealInputDevices::apply")
    }
}
