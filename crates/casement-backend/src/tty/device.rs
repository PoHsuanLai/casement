//! The display devices the tty backend holds (P1-Z; filled by P1-D2): one `Kms` per device
//! node, opened through the injected opener, with the connectors each reported.

use super::capture_gpu::CaptureGpu;
use super::hotplug::{KnownConnectors, reconcile};
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
    /// Whether the start-up announcement has been made (see [`TtyDevices::announce`]). Until it
    /// is, connector events are held back, and the announcement reports every connector known at
    /// that moment; after it they are reported as they happen.
    pub announced: bool,
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

    /// A device appeared: open it and report its connectors as plugged. An already open node is
    /// reconciled instead (a repeated `Added` loses and repeats nothing).
    pub fn added(
        &mut self,
        path: DevicePath,
        opener: &mut dyn KmsOpener,
    ) -> Result<Vec<ConnectorEvent>, KmsError> {
        if self.devices.contains_key(&path) {
            return self.changed(&path);
        }
        let kms = opener.open(&path)?;
        let (known, events) = reconcile(&KnownConnectors::default(), &kms.resources());
        self.devices.insert(path, OpenDevice { kms, known });
        Ok(self.released(events))
    }

    /// A device changed: reconcile its connectors.
    pub fn changed(&mut self, path: &DevicePath) -> Result<Vec<ConnectorEvent>, KmsError> {
        let device = self
            .devices
            .get_mut(path)
            .ok_or_else(|| KmsError::DeviceGone(path.0.clone()))?;
        // The device's own change hints are drained; the resources are the truth.
        let _ = device.kms.events();
        let (known, events) = reconcile(&device.known, &device.kms.resources());
        device.known = known;
        Ok(self.released(events))
    }

    /// A device went away: its connectors are unplugged.
    pub fn removed(&mut self, path: &DevicePath) -> Vec<ConnectorEvent> {
        let events = self
            .devices
            .remove(path)
            .map(|device| {
                device
                    .known
                    .0
                    .into_values()
                    .map(|facts| ConnectorEvent::Unplugged(facts.key))
                    .collect()
            })
            .unwrap_or_default();
        self.released(events)
    }

    /// The device that drives the connector `name`.
    pub fn driving(&mut self, name: &OutputName) -> Option<&mut Box<dyn Kms>> {
        self.devices
            .values_mut()
            .find(|device| device.known.0.values().any(|facts| &facts.name == name))
            .map(|device| &mut device.kms)
    }

    /// The renderer of the first open device that lends one, to draw captures with. With more
    /// than one GPU the first by node path answers every capture (a client buffer on another
    /// GPU fails over to the software renderer).
    pub fn capture_gpu(&self) -> Option<Box<dyn CaptureGpu>> {
        self.devices
            .values()
            .find_map(|device| device.kms.capture_gpu())
    }

    /// The start-up announcement, once: a plug for every connector of every open device, in
    /// device and connector order. Until it is made, `added`, `changed` and `removed` report
    /// nothing (what they saw is in the announcement); after it, it reports nothing.
    pub fn announce(&mut self) -> Vec<ConnectorEvent> {
        match self.announced {
            true => Vec::new(),
            false => {
                self.announced = true;
                self.devices
                    .values()
                    .flat_map(|device| device.known.0.values())
                    .cloned()
                    .map(ConnectorEvent::Plugged)
                    .collect()
            }
        }
    }

    /// `events` when the announcement is made, else nothing.
    fn released(&self, events: Vec<ConnectorEvent>) -> Vec<ConnectorEvent> {
        match self.announced {
            true => events,
            false => Vec::new(),
        }
    }
}
