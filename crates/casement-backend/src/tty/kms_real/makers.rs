//! The two ways a [`RealKms`] makes a scanout surface (feature `tty`): GLES on the GPU and
//! pixman over dumb buffers. [`Routes`](super::Routes) decides which to ask for; a child module
//! of `kms_real` because both read the device's private state.

use super::super::kms::{ConnectorId, KmsError};
use super::super::render_mode::ScanoutMaker;
use super::super::soft_real::{SharedPixman, SoftScanout, SoftShare};
use super::super::surface::ScanoutSurface;
use super::super::surface_real::{DeviceShare, RealScanout};
use super::RealKms;
use casement_core::output::OutputConfig;
use smithay::backend::drm::DrmSurface;
use smithay::backend::renderer::pixman::PixmanRenderer;
use std::cell::RefCell;
use std::rc::Rc;

/// A device that makes surfaces for one output.
pub(super) struct Makers<'a> {
    /// The device.
    pub(super) kms: &'a mut RealKms,
}

impl RealKms {
    /// The surface lit for `connector`, not yet handed to a scanout. A surface a failed GLES
    /// attempt used up is lit again, so the CPU can take its place.
    fn surface_for(&mut self, connector: ConnectorId) -> Result<DrmSurface, KmsError> {
        if let Some(surface) = self.surfaces.remove(&connector) {
            return Ok(surface);
        }
        let output = self
            .lit
            .get(&connector)
            .cloned()
            .ok_or(KmsError::UnknownConnector(connector))?;
        self.light(&output)?;
        self.surfaces
            .remove(&connector)
            .ok_or(KmsError::UnknownConnector(connector))
    }

    /// The CPU's renderer, made on first use.
    fn pixman(&mut self) -> Result<SharedPixman, KmsError> {
        if let Some(renderer) = &self.pixman {
            return Ok(Rc::clone(renderer));
        }
        let renderer = PixmanRenderer::new()
            .map_err(|error| KmsError::Io(std::io::Error::other(error.to_string())))?;
        let shared = Rc::new(RefCell::new(renderer));
        self.pixman = Some(Rc::clone(&shared));
        Ok(shared)
    }
}

impl ScanoutMaker for Makers<'_> {
    fn gpu(
        &mut self,
        connector: ConnectorId,
        config: &OutputConfig,
    ) -> Result<Box<dyn ScanoutSurface>, KmsError> {
        let kms = &mut *self.kms;
        // The renderer first: a GPU that cannot start leaves the surface for the CPU.
        let feed = kms.feed()?;
        let surface = kms.surface_for(connector)?;
        let claim = Rc::new(());
        kms.out.insert(connector, Rc::downgrade(&claim));
        let scanout = RealScanout::open(
            connector,
            surface,
            DeviceShare {
                gbm: kms.gbm()?,
                feed,
                cursor: kms.device.cursor_size(),
                flips: Rc::clone(&kms.flips),
            },
            claim,
            config,
        )?;
        Ok(Box::new(scanout))
    }

    fn software(
        &mut self,
        connector: ConnectorId,
        config: &OutputConfig,
    ) -> Result<Box<dyn ScanoutSurface>, KmsError> {
        let kms = &mut *self.kms;
        let renderer = kms.pixman()?;
        let surface = kms.surface_for(connector)?;
        let claim = Rc::new(());
        kms.out.insert(connector, Rc::downgrade(&claim));
        let scanout = SoftScanout::open(
            connector,
            surface,
            SoftShare {
                fd: kms.fd.clone(),
                renderer,
                cursor: kms.device.cursor_size(),
                flips: Rc::clone(&kms.flips),
            },
            claim,
            config,
        )?;
        Ok(Box::new(scanout))
    }
}
