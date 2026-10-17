//! The fake device's two kinds of scanout surface (CASB-07): the fake GPU's, which a test can
//! make refuse, and the CPU's. Both are a [`FakeScanout`] that reports each frame at the next
//! look; what differs is what the test reads afterwards (which way each connector was drawn and
//! how often the GPU was asked).

use super::super::scanout::{FakeScanout, GlassHandle};
use casement_backend::tty::{ConnectorId, KmsError, ScanoutMaker, ScanoutSurface};
use casement_core::output::OutputConfig;
use std::collections::{BTreeMap, BTreeSet};

/// Borrows the parts of a [`KmsWorld`](super::KmsWorld) a surface is made from.
pub(super) struct FakeMaker<'a> {
    /// The connectors whose GPU surface fails.
    pub(super) refuses: &'a BTreeSet<ConnectorId>,
    /// How many times the GPU was asked for a surface.
    pub(super) asked: &'a mut u32,
    /// The surface last handed out for each connector.
    pub(super) glass: &'a mut BTreeMap<ConnectorId, GlassHandle>,
}

impl FakeMaker<'_> {
    fn hand_out(&mut self, connector: ConnectorId) -> Box<dyn ScanoutSurface> {
        let glass = GlassHandle::new(FakeScanout::following());
        self.glass.insert(connector, glass.clone());
        Box::new(glass)
    }
}

impl ScanoutMaker for FakeMaker<'_> {
    fn gpu(
        &mut self,
        connector: ConnectorId,
        config: &OutputConfig,
    ) -> Result<Box<dyn ScanoutSurface>, KmsError> {
        let _ = config;
        *self.asked += 1;
        match self.refuses.contains(&connector) {
            true => Err(KmsError::Io(std::io::Error::other("the fake GPU refuses"))),
            false => Ok(self.hand_out(connector)),
        }
    }

    fn software(
        &mut self,
        connector: ConnectorId,
        config: &OutputConfig,
    ) -> Result<Box<dyn ScanoutSurface>, KmsError> {
        let _ = config;
        Ok(self.hand_out(connector))
    }
}
