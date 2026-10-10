//! The display devices the tty backend holds (P1-Z; filled by P1-D2): one `Kms` per device
//! node, opened through the injected opener, with the connectors each reported.

use super::hotplug::KnownConnectors;
use super::kms::{Kms, KmsError, KmsOpener};
use super::udev::DevicePath;
use crate::headless::ConnectorEvent;
use casement_core::OutputName;
use std::collections::BTreeMap;

/// Every open device.
#[derive(Debug, Default)]
pub struct TtyDevices {
    /// Each open device by its node.
    pub devices: BTreeMap<DevicePath, OpenDevice>,
}

/// One open device and its connectors.
#[derive(Debug)]
pub struct OpenDevice {
    /// The device.
    pub kms: Box<dyn Kms>,
    /// The connectors it last reported plugged.
    pub known: KnownConnectors,
}

impl TtyDevices {
    /// No device open.
    pub fn empty() -> TtyDevices {
        TtyDevices::default()
    }

    /// A device appeared: open it and report its connectors as plugged.
    pub fn added(
        &mut self,
        path: DevicePath,
        opener: &mut dyn KmsOpener,
    ) -> Result<Vec<ConnectorEvent>, KmsError> {
        let _ = (&mut *self, path, opener);
        todo!("P1-D2: TtyDevices::added")
    }

    /// A device changed: reconcile its connectors.
    pub fn changed(&mut self, path: &DevicePath) -> Result<Vec<ConnectorEvent>, KmsError> {
        let _ = (&mut *self, path);
        todo!("P1-D2: TtyDevices::changed")
    }

    /// A device went away: its connectors are unplugged.
    pub fn removed(&mut self, path: &DevicePath) -> Vec<ConnectorEvent> {
        let _ = (&mut *self, path);
        todo!("P1-D2: TtyDevices::removed")
    }

    /// The device that drives the connector `name`.
    pub fn driving(&mut self, name: &OutputName) -> Option<&mut Box<dyn Kms>> {
        let _ = (&mut *self, name);
        todo!("P1-D2: TtyDevices::driving")
    }
}
