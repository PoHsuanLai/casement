//! What draws an output: the GPU, or the CPU alone (CASB-07). Pure; the real surfaces are
//! `surface_real.rs` (GLES) and `soft_real.rs` (pixman over dumb buffers).
//!
//! `casement --safe` has to survive a broken GPU driver, so in [`RenderMode::Software`] no GBM
//! device, EGL display or GLES context is ever opened: every output is drawn by pixman into dumb
//! buffers the kernel scans out. In [`RenderMode::Gpu`] each output asks for a GLES surface; one
//! the driver cannot make is drawn by the CPU instead, that output alone, and the log says so
//! once. An output that failed is not asked again for the rest of the run: a driver that refused
//! once is not trusted with a second try at every mode change or VT switch.

use super::kms::{ConnectorId, KmsError};
use super::surface::ScanoutSurface;
use casement_core::output::OutputConfig;
use std::collections::BTreeSet;

/// How a session draws its outputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum RenderMode {
    /// With the GPU (GLES), falling back to the CPU for an output whose GPU surface fails.
    #[default]
    Gpu,
    /// With the CPU alone (`--safe`): the GPU is never opened.
    Software,
}

/// What one scanout surface draws with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Drawing {
    /// GLES on a GPU.
    Gpu,
    /// pixman on the CPU, into dumb buffers.
    Pixman,
}

/// Makes the two kinds of surface for a device. Real: `kms_real.rs`; fake: the test kit's
/// `FakeKms`.
pub trait ScanoutMaker {
    /// A surface drawn by GLES on the GPU. May fail: a driver that cannot start.
    fn gpu(
        &mut self,
        connector: ConnectorId,
        config: &OutputConfig,
    ) -> Result<Box<dyn ScanoutSurface>, KmsError>;

    /// A surface drawn by pixman into dumb buffers. Opens no GPU userspace at all.
    fn software(
        &mut self,
        connector: ConnectorId,
        config: &OutputConfig,
    ) -> Result<Box<dyn ScanoutSurface>, KmsError>;
}

/// A surface and how it draws.
#[derive(Debug)]
#[non_exhaustive]
pub struct Opened {
    /// The surface.
    pub surface: Box<dyn ScanoutSurface>,
    /// What draws it.
    pub drawing: Drawing,
}

/// Which way each output of a device is drawn.
///
/// ```
/// use casement_backend::tty::{RenderMode, Routes};
///
/// assert_eq!(Routes::new(RenderMode::Software).mode(), RenderMode::Software);
/// assert_eq!(Routes::default().mode(), RenderMode::Gpu);
/// ```
#[derive(Debug, Default)]
pub struct Routes {
    mode: RenderMode,
    /// The connectors whose GPU surface failed: drawn by the CPU from then on.
    failed: BTreeSet<ConnectorId>,
}

impl Routes {
    /// Routes for a session drawing in `mode`.
    pub fn new(mode: RenderMode) -> Routes {
        Routes {
            mode,
            failed: BTreeSet::new(),
        }
    }

    /// How the session draws.
    pub fn mode(&self) -> RenderMode {
        self.mode
    }

    /// The surface for `connector`: pixman in software mode and for an output whose GPU surface
    /// failed before; else GLES, with pixman if GLES fails now (said once, in the log).
    pub fn open(
        &mut self,
        maker: &mut dyn ScanoutMaker,
        connector: ConnectorId,
        config: &OutputConfig,
    ) -> Result<Opened, KmsError> {
        if self.mode == RenderMode::Software || self.failed.contains(&connector) {
            return by_cpu(maker, connector, config);
        }
        match maker.gpu(connector, config) {
            Ok(surface) => Ok(Opened {
                surface,
                drawing: Drawing::Gpu,
            }),
            Err(error) => {
                self.failed.insert(connector);
                tracing::warn!(?connector, %error, "the GPU could not draw this output; the CPU draws it instead (slower, no hardware cursor)");
                by_cpu(maker, connector, config)
            }
        }
    }
}

/// The CPU's surface for `connector`.
fn by_cpu(
    maker: &mut dyn ScanoutMaker,
    connector: ConnectorId,
    config: &OutputConfig,
) -> Result<Opened, KmsError> {
    maker.software(connector, config).map(|surface| Opened {
        surface,
        drawing: Drawing::Pixman,
    })
}
