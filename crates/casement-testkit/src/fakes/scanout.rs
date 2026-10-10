//! The fake scanout surface (P1-D3): `FakeScanout` takes queued frames and reports them
//! presented on a script of vblanks, so the scheduler and the flip bookkeeping run with no
//! device. Its frames land in a pixman image per output (the FakeKms suite reads them back).

use casement_backend::tty::{Presented, QueueOutcome, QueuedFrame, ScanoutSurface};
use std::collections::VecDeque;

/// A scripted scanout.
#[derive(Debug, Default)]
pub struct FakeScanout {
    /// The presentations the surface will report, oldest first.
    pub script: VecDeque<Presented>,
    /// How many frames were queued.
    pub queued: usize,
}

impl FakeScanout {
    /// A surface that will report `script`.
    pub fn scripted(script: Vec<Presented>) -> FakeScanout {
        FakeScanout {
            script: script.into(),
            queued: 0,
        }
    }
}

impl ScanoutSurface for FakeScanout {
    fn queue(&mut self, frame: QueuedFrame) -> QueueOutcome {
        let _ = frame;
        todo!("P1-D3: FakeScanout::queue")
    }

    fn presented(&mut self) -> Option<Presented> {
        todo!("P1-D3: FakeScanout::presented")
    }
}
