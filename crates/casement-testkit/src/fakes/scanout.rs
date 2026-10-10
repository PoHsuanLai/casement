//! The fake scanout surface (P1-D3): `FakeScanout` takes queued frames and reports them
//! presented on a script of vblanks, so the scheduler and the flip bookkeeping run with no
//! device.

use casement_backend::tty::{Presented, QueueOutcome, QueuedFrame, ScanoutSurface};
use casement_core::frame::{CursorPlaneCaps, CursorPlaneChoice, cursor_plane_for};
use casement_core::geometry::PhysicalSize;
use casement_core::time::VblankSeq;
use ds_core::geometry::scale::Scale;
use std::collections::VecDeque;

/// A scripted scanout. One flip is in flight at a time, as on a device: queueing again before
/// `presented` answers [`QueueOutcome::Busy`].
///
/// ```no_run
/// use casement_backend::tty::{Presented, QueueOutcome, QueuedFrame, ScanoutSurface};
/// use casement_core::time::{Mono, VblankSeq};
/// use casement_testkit::fakes::FakeScanout;
/// use std::time::Duration;
///
/// let at = Presented::new(VblankSeq(0), Mono(Duration::ZERO));
/// let mut surface = FakeScanout::scripted(vec![at]);
/// let frame = QueuedFrame::Rendered(casement_backend::headless::FramePixels::Dummy);
/// assert!(matches!(surface.queue(frame), QueueOutcome::Queued(VblankSeq(0))));
/// assert_eq!(surface.presented(), Some(at));
/// ```
#[derive(Debug)]
pub struct FakeScanout {
    /// The presentations the surface will report, oldest first.
    pub script: VecDeque<Presented>,
    /// How many frames were queued.
    pub queued: usize,
    /// The vblank the next queued frame is for (the test moves it as its clock advances).
    pub vblank: VblankSeq,
    /// What this output's cursor plane can do (none by default).
    pub cursor: CursorPlaneCaps,
    flying: Option<VblankSeq>,
}

impl FakeScanout {
    /// A surface that will report `script`.
    pub fn scripted(script: Vec<Presented>) -> FakeScanout {
        FakeScanout {
            script: script.into(),
            queued: 0,
            vblank: VblankSeq(0),
            cursor: CursorPlaneCaps::new(None),
            flying: None,
        }
    }

    /// The same surface with a cursor plane of `caps`.
    #[must_use]
    pub fn with_cursor(self, caps: CursorPlaneCaps) -> FakeScanout {
        FakeScanout {
            cursor: caps,
            ..self
        }
    }

    /// Where a cursor of `size` (logical) at `scale` would be drawn on this surface.
    pub fn cursor_choice(&self, size: PhysicalSize, scale: Scale) -> CursorPlaneChoice {
        cursor_plane_for(&self.cursor, size, scale)
    }
}

impl ScanoutSurface for FakeScanout {
    fn queue(&mut self, frame: QueuedFrame<'_>) -> QueueOutcome {
        let _ = frame;
        match self.flying {
            Some(_) => QueueOutcome::Busy,
            None => {
                self.queued += 1;
                self.flying = Some(self.vblank);
                QueueOutcome::Queued(self.vblank)
            }
        }
    }

    fn presented(&mut self) -> Option<Presented> {
        self.flying?;
        let next = self.script.pop_front()?;
        self.flying = None;
        Some(next)
    }
}
