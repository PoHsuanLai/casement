//! The fake libinput configuration: `FakeInputDevices` records the settings applied to each
//! device, so a test asserts them at the seam (P1-I1). A clone shares the record.

use casement_backend::input::{Applied, DeviceSettings, InputDevices};
use casement_core::DeviceId;
use std::sync::{Arc, Mutex, PoisonError};

/// A recorder standing in for libinput's device configuration.
#[derive(Debug, Clone, Default)]
pub struct FakeInputDevices {
    applied: Arc<Mutex<Vec<(DeviceId, DeviceSettings)>>>,
}

impl FakeInputDevices {
    /// A recorder with nothing applied.
    pub fn new() -> FakeInputDevices {
        FakeInputDevices::default()
    }

    /// Every application so far, in order.
    pub fn applied(&self) -> Vec<(DeviceId, DeviceSettings)> {
        self.applied
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl InputDevices for FakeInputDevices {
    fn apply(&mut self, device: DeviceId, settings: &DeviceSettings) -> Applied {
        self.applied
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push((device, *settings));
        Applied::All
    }
}
