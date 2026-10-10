//! `FakeKms`: `Kms` over a shared world a test scripts.

use casement_backend::headless::ConnectorEvent;
use casement_backend::tty::{
    CommitVerdict, ConnectorFacts, ConnectorId, DevicePath, Kms, KmsCommit, KmsError, KmsOpener,
    KmsResources,
};
use casement_core::output::{GammaRamp, OutputPower};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

/// What the fake device is and has been told.
#[derive(Debug, Default)]
pub struct KmsWorld {
    /// What the device offers now.
    pub resources: KmsResources,
    /// The commits applied, in order.
    pub commits: Vec<KmsCommit>,
    /// The power each connector was last given.
    pub power: BTreeMap<ConnectorId, OutputPower>,
    /// The gamma each connector was last given.
    pub gamma: BTreeMap<ConnectorId, GammaRamp>,
    /// The connector events not yet taken by `events`.
    pub pending: Vec<ConnectorEvent>,
}

/// A fake display device.
#[derive(Debug, Clone, Default)]
pub struct FakeKms {
    /// The world, shared with every clone.
    pub world: Arc<Mutex<KmsWorld>>,
}

impl FakeKms {
    /// A device offering `resources`.
    pub fn with(resources: KmsResources) -> FakeKms {
        FakeKms {
            world: Arc::new(Mutex::new(KmsWorld {
                resources,
                ..KmsWorld::default()
            })),
        }
    }

    /// A monitor is plugged: the connector appears and its event is queued.
    pub fn plug(&self, connector: ConnectorFacts) {
        let _ = connector;
        todo!("P1-D2: FakeKms::plug")
    }

    /// The monitor on `connector` is unplugged.
    pub fn unplug(&self, connector: ConnectorId) {
        let _ = connector;
        todo!("P1-D2: FakeKms::unplug")
    }

    /// The commits applied so far.
    pub fn commits(&self) -> Vec<KmsCommit> {
        todo!("P1-D2: FakeKms::commits")
    }
}

impl Kms for FakeKms {
    fn resources(&self) -> KmsResources {
        todo!("P1-D2: FakeKms::resources")
    }

    fn test(&mut self, commit: &KmsCommit) -> CommitVerdict {
        let _ = commit;
        todo!("P1-D2: FakeKms::test")
    }

    fn commit(&mut self, commit: KmsCommit) -> Result<(), KmsError> {
        let _ = commit;
        todo!("P1-D2: FakeKms::commit")
    }

    fn set_power(&mut self, connector: ConnectorId, power: OutputPower) -> Result<(), KmsError> {
        let _ = (connector, power);
        todo!("P1-D2: FakeKms::set_power")
    }

    fn set_gamma(&mut self, connector: ConnectorId, ramp: &GammaRamp) -> Result<(), KmsError> {
        let _ = (connector, ramp);
        todo!("P1-D2: FakeKms::set_gamma")
    }

    fn events(&mut self) -> Vec<ConnectorEvent> {
        todo!("P1-D2: FakeKms::events")
    }
}

/// Opens the fake devices a test registered, by path.
#[derive(Debug, Default)]
pub struct FakeKmsOpener {
    /// The devices that can be opened.
    pub devices: BTreeMap<DevicePath, FakeKms>,
}

impl KmsOpener for FakeKmsOpener {
    fn open(&mut self, path: &DevicePath) -> Result<Box<dyn Kms>, KmsError> {
        let _ = path;
        todo!("P1-D2: FakeKmsOpener::open")
    }
}
