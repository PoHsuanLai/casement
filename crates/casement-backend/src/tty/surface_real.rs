//! The real scanout surface (feature `tty`): `ScanoutSurface` over the device crates'
//! compositor, a thin adapter (B7). P1-Z declares it; P1-D3 fills it.

use super::kms::{ConnectorId, CrtcId};
use super::surface::{Presented, QueueOutcome, QueuedFrame, ScanoutSurface};

/// One output's scanout surface.
#[derive(Debug)]
pub struct RealScanout {
    /// The connector it scans out to.
    pub connector: ConnectorId,
    /// The CRTC that drives it.
    pub crtc: CrtcId,
}

impl ScanoutSurface for RealScanout {
    fn queue(&mut self, frame: QueuedFrame) -> QueueOutcome {
        let _ = frame;
        todo!("P1-D3: RealScanout::queue")
    }

    fn presented(&mut self) -> Option<Presented> {
        todo!("P1-D3: RealScanout::presented")
    }
}
