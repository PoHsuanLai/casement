//! The real KMS device (feature `tty`): `Kms` over the device crates, a thin adapter (B7).
//! P1-Z declares it; P1-D2 fills it. Nothing else in the workspace names the device crates.

use super::kms::{CommitVerdict, ConnectorId, Kms, KmsCommit, KmsError, KmsOpener, KmsResources};
use super::udev::DevicePath;
use crate::headless::ConnectorEvent;
use casement_core::output::{GammaRamp, OutputPower};
use std::os::fd::OwnedFd;

/// A display device opened through the session.
#[derive(Debug)]
pub struct RealKms {
    /// The device's file descriptor, until P1-D2 wraps it in the device crates' types.
    pub fd: OwnedFd,
}

impl RealKms {
    /// Wraps the device behind `fd`: reads its resources and takes master.
    pub fn open(fd: OwnedFd) -> Result<RealKms, KmsError> {
        let _ = fd;
        todo!("P1-D2: RealKms::open")
    }
}

impl Kms for RealKms {
    fn resources(&self) -> KmsResources {
        todo!("P1-D2: RealKms::resources")
    }

    fn test(&mut self, commit: &KmsCommit) -> CommitVerdict {
        let _ = commit;
        todo!("P1-D2: RealKms::test")
    }

    fn commit(&mut self, commit: KmsCommit) -> Result<(), KmsError> {
        let _ = commit;
        todo!("P1-D2: RealKms::commit")
    }

    fn set_power(&mut self, connector: ConnectorId, power: OutputPower) -> Result<(), KmsError> {
        let _ = (connector, power);
        todo!("P1-D2: RealKms::set_power")
    }

    fn set_gamma(&mut self, connector: ConnectorId, ramp: &GammaRamp) -> Result<(), KmsError> {
        let _ = (connector, ramp);
        todo!("P1-D2: RealKms::set_gamma")
    }

    fn events(&mut self) -> Vec<ConnectorEvent> {
        todo!("P1-D2: RealKms::events")
    }
}

/// Opens devices through a session (`session_real.rs` hands the descriptors).
#[derive(Debug, Default)]
pub struct RealKmsOpener;

impl KmsOpener for RealKmsOpener {
    fn open(&mut self, path: &DevicePath) -> Result<Box<dyn Kms>, KmsError> {
        let _ = path;
        todo!("P1-D2: RealKmsOpener::open")
    }
}
