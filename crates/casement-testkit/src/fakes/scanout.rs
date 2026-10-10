//! The fake scanout surface (P1-D3): `FakeScanout` takes queued frames and reports them
//! presented on a script of vblanks, so the scheduler and the flip bookkeeping run with no
//! device. P1-T adds [`FakeScanout::following`], which presents each frame at the next look,
//! and [`GlassHandle`], the shared view a [`FakeKms`](super::FakeKms) gives a test of the
//! surface it handed the backend.

use casement_backend::tty::{KmsError, Presented, QueueOutcome, QueuedFrame, ScanoutSurface};
use casement_core::frame::{CursorPlaneCaps, CursorPlaneChoice, cursor_plane_for};
use casement_core::geometry::PhysicalSize;
use casement_core::time::{Mono, VblankSeq};
use ds_core::geometry::scale::Scale;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

/// How the surface decides a frame reached the glass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum Pacing {
    /// Only what the test scripted is reported (the scheduler tests).
    #[default]
    Scripted,
    /// A queued frame is reported at the next look, at the vblank it was queued for.
    Following,
}

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
/// assert_eq!(surface.presented(Mono(Duration::ZERO)), Some(at));
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
    /// How many times the seat came back and the surface was told to forget the screen.
    pub resets: usize,
    /// How many times the output was told to go dark.
    pub blanks: usize,
    /// How frames are reported.
    pub pacing: Pacing,
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
            resets: 0,
            blanks: 0,
            pacing: Pacing::Scripted,
            flying: None,
        }
    }

    /// A surface that reports each frame at the next look, at the vblank it was queued for; the
    /// vblank after that is the next one.
    pub fn following() -> FakeScanout {
        FakeScanout {
            pacing: Pacing::Following,
            ..FakeScanout::scripted(Vec::new())
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

    /// Whether a flip is in flight.
    pub fn flying(&self) -> Option<VblankSeq> {
        self.flying
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

    fn presented(&mut self, now: Mono) -> Option<Presented> {
        let flying = self.flying?;
        match self.pacing {
            Pacing::Scripted => {
                let next = self.script.pop_front()?;
                self.flying = None;
                Some(next)
            }
            Pacing::Following => {
                self.flying = None;
                self.vblank = VblankSeq(flying.0.saturating_add(1));
                Some(Presented::new(flying, now))
            }
        }
    }

    fn blank(&mut self) -> Result<(), KmsError> {
        self.blanks += 1;
        self.flying = None;
        Ok(())
    }

    fn reset(&mut self) {
        self.resets += 1;
        self.flying = None;
    }
}

/// A [`FakeScanout`] a test and a backend both hold: the backend takes it as a
/// `Box<dyn ScanoutSurface>`, the test reads what was queued.
#[derive(Debug, Clone)]
pub struct GlassHandle(Arc<Mutex<FakeScanout>>);

impl GlassHandle {
    /// Shares `surface`.
    pub fn new(surface: FakeScanout) -> GlassHandle {
        GlassHandle(Arc::new(Mutex::new(surface)))
    }

    /// The surface, locked.
    pub fn surface(&self) -> MutexGuard<'_, FakeScanout> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// How many frames were queued.
    pub fn queued(&self) -> usize {
        self.surface().queued
    }

    /// How many times the output was told to go dark.
    pub fn blanks(&self) -> usize {
        self.surface().blanks
    }

    /// How many times the surface was told the seat came back.
    pub fn resets(&self) -> usize {
        self.surface().resets
    }
}

impl ScanoutSurface for GlassHandle {
    fn queue(&mut self, frame: QueuedFrame<'_>) -> QueueOutcome {
        self.surface().queue(frame)
    }

    fn presented(&mut self, now: Mono) -> Option<Presented> {
        self.surface().presented(now)
    }

    fn blank(&mut self) -> Result<(), KmsError> {
        self.surface().blank()
    }

    fn reset(&mut self) {
        self.surface().reset();
    }
}
