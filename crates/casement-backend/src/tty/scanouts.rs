//! The scanout of every lit output (P1-D3): a surface, its pending flip, its vblank clock and
//! the render-cost history the frame scheduler reads.

use super::flip::FlipTracker;
use super::kms::KmsError;
use super::surface::{Presented, QueueOutcome, QueuedFrame, ScanoutSurface};
use super::watch::{Failing, FailureRun, FlipWatch, Health, Level, Recoveries};
use crate::backend::BackendError;
use crate::headless::FrameSource;
use casement_core::OutputName;
use casement_core::frame::{Deadline, FrameOutcome, RenderHistory, next_deadline, predict};
use casement_core::time::{Mono, VblankClock, VblankSeq};
use std::collections::BTreeMap;
use std::time::Duration;

/// How many frames an output has shown and how many were late.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct FrameTally {
    /// Frames that reached the glass.
    pub presented: u64,
    /// Of those, frames that missed the vblank they were made for.
    pub missed: u64,
}

/// What asking an output for a frame came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PresentOutcome {
    /// A frame was queued for this vblank; its flip will be reported.
    Flipped(VblankSeq),
    /// The frame changed nothing on the glass, so none was queued: clients still get their
    /// frame callbacks, but there is no presentation to report.
    Unchanged,
    /// Nothing was drawn: a flip is pending, the seat is lent, the output is dark or unlit.
    Waiting,
    /// A flip never landed, or frames kept failing, and the surface was made to forget the
    /// screen: the next frame draws everything again. Nothing was queued now.
    Recovered,
    /// The same went on after a recovery: the output must be set up again from its
    /// configuration (`TtyNote::Reinit`). Nothing was queued.
    Stalled,
    /// The device refused a frame because it is no longer the display master (the seat was
    /// taken back without a word): it must take master again before anything is drawn on it.
    MasterLost,
}

impl From<Option<VblankSeq>> for PresentOutcome {
    fn from(shown: Option<VblankSeq>) -> PresentOutcome {
        shown.map_or(PresentOutcome::Waiting, PresentOutcome::Flipped)
    }
}

/// One output's scanout.
#[derive(Debug)]
pub struct Scanout {
    surface: Box<dyn ScanoutSurface>,
    clock: VblankClock,
    flip: FlipTracker,
    history: RenderHistory,
    tally: FrameTally,
    /// When the flip in flight was queued, and for how long nothing has landed.
    watch: FlipWatch,
    /// Frames that failed one after another.
    failures: FailureRun,
    /// How lately the output was set going again.
    recoveries: Recoveries,
}

impl Scanout {
    /// A scanout over `surface`, whose vblanks follow `clock`.
    pub fn new(surface: Box<dyn ScanoutSurface>, clock: VblankClock) -> Scanout {
        Scanout {
            surface,
            clock,
            flip: FlipTracker::new(),
            history: RenderHistory::empty(),
            tally: FrameTally::default(),
            watch: FlipWatch::default(),
            failures: FailureRun::default(),
            recoveries: Recoveries::default(),
        }
    }

    /// The same scanout, remembering how long the output it replaces had shown nothing.
    #[must_use]
    pub fn carrying(self, watch: FlipWatch) -> Scanout {
        Scanout { watch, ..self }
    }

    /// How long this output has been waiting for a frame to land.
    pub fn watch(&self) -> FlipWatch {
        self.watch
    }

    /// Whether this output has shown nothing for too long at `now`.
    pub fn starved(&self, now: Mono) -> bool {
        self.watch.starved(now)
    }

    /// Takes in what reached the glass since the last call (`now` is this look's instant on the
    /// session clock); the outcomes, oldest first.
    pub fn settle(&mut self, now: Mono) -> Vec<(Presented, FrameOutcome)> {
        std::iter::from_fn(|| self.surface.presented(now))
            .map(|presented| {
                let (flip, outcome) = self.flip.presented(presented);
                self.flip = flip;
                self.watch = self.watch.landed();
                self.tally.presented = self.tally.presented.saturating_add(1);
                if matches!(outcome, FrameOutcome::Missed { .. }) {
                    self.tally.missed = self.tally.missed.saturating_add(1);
                }
                (presented, outcome)
            })
            .collect()
    }

    /// Draws `source` and queues it: the vblank it is for, unchanged, or waiting while a flip is
    /// pending.
    pub fn present(
        &mut self,
        source: &dyn FrameSource,
        now: Mono,
    ) -> Result<PresentOutcome, BackendError> {
        let _ = self.settle(now);
        if self.flip.wanted.is_some() {
            return Ok(match self.watch.flip_lost(now) {
                false => PresentOutcome::Waiting,
                true => {
                    tracing::warn!("a page flip never landed; the output draws its picture again");
                    self.recover(now)
                }
            });
        }
        match self.surface.queue(QueuedFrame::Draw(source)) {
            QueueOutcome::Queued(wanted) => {
                self.flip = self.flip.queued(wanted);
                self.watch = self.watch.queued(now);
                self.failures = self.failures.succeeded();
                Ok(PresentOutcome::Flipped(wanted))
            }
            QueueOutcome::Busy => Ok(PresentOutcome::Waiting),
            QueueOutcome::Unchanged => {
                self.failures = self.failures.succeeded();
                Ok(PresentOutcome::Unchanged)
            }
            QueueOutcome::Failed(error) if lost_master(&error) => {
                self.surface.reset();
                self.flip = FlipTracker::new();
                self.watch = self.watch.given_up();
                Ok(PresentOutcome::MasterLost)
            }
            QueueOutcome::Failed(error) => {
                self.watch = self.watch.asked(now);
                let (run, verdict) = self.failures.failed();
                self.failures = run;
                match verdict {
                    Failing::Carry => Err(error.into()),
                    Failing::Reinit => {
                        tracing::warn!(%error, "frames keep failing; the output draws its picture again");
                        Ok(self.recover(now))
                    }
                }
            }
        }
    }

    /// The flip in flight (or the run of failures) is given up: the surface forgets the screen,
    /// and the output is reset, or after a recent reset set up again.
    fn recover(&mut self, now: Mono) -> PresentOutcome {
        self.surface.reset();
        self.flip = FlipTracker::new();
        self.watch = self.watch.given_up();
        self.failures = FailureRun::default();
        let (recoveries, level) = self.recoveries.next(now);
        self.recoveries = recoveries;
        match level {
            Level::Reset => PresentOutcome::Recovered,
            Level::Reinit => PresentOutcome::Stalled,
        }
    }

    /// The output goes dark: the surface stops scanning out until the next frame.
    pub fn blank(&mut self) -> Result<(), KmsError> {
        self.flip = FlipTracker::new();
        self.watch = self.watch.idle();
        self.surface.blank()
    }

    /// The seat came back: the surface forgets the screen and no flip is pending any more.
    pub fn reset(&mut self) {
        self.surface.reset();
        self.flip = FlipTracker::new();
        self.watch = self.watch.idle();
    }

    /// Records what the frame just drawn cost, for the next prediction.
    pub fn record_cost(&mut self, cost: Duration) {
        self.history = std::mem::take(&mut self.history).with(cost);
    }

    /// When to start the next frame, given `now` and `slack`.
    pub fn deadline(&self, now: Mono, slack: Duration) -> Deadline {
        next_deadline(&self.clock, predict(&self.history), now, slack)
    }

    /// The frames shown and missed so far.
    pub fn tally(&self) -> FrameTally {
        self.tally
    }
}

/// Whether a refused frame means the device lost the display master: the kernel answers a
/// device that is no longer master with "permission denied".
fn lost_master(error: &KmsError) -> bool {
    matches!(error, KmsError::Io(io) if io.kind() == std::io::ErrorKind::PermissionDenied)
}

/// Whether frames flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum Flow {
    /// The seat is ours: frames are queued.
    #[default]
    Running,
    /// Another session holds the seat: nothing is queued.
    Held,
}

/// Every output's scanout.
#[derive(Debug, Default)]
pub struct ScanoutSet {
    outputs: BTreeMap<OutputName, Scanout>,
    flow: Flow,
    /// What the scanouts that went away had shown, so the totals only grow.
    retired: FrameTally,
}

impl ScanoutSet {
    /// No output lit.
    pub fn empty() -> ScanoutSet {
        ScanoutSet::default()
    }

    /// `name` is lit and scans out through `scanout`.
    pub fn attach(&mut self, name: OutputName, scanout: Scanout) {
        self.outputs.insert(name, scanout);
    }

    /// `name` went dark or away.
    pub fn detach(&mut self, name: &OutputName) -> Option<Scanout> {
        let gone = self.outputs.remove(name)?;
        let tally = gone.tally();
        self.retired = FrameTally {
            presented: self.retired.presented.saturating_add(tally.presented),
            missed: self.retired.missed.saturating_add(tally.missed),
        };
        Some(gone)
    }

    /// The frames shown and missed over the run on every output, those that went away included.
    pub fn total(&self) -> FrameTally {
        self.outputs
            .values()
            .map(Scanout::tally)
            .fold(self.retired, |sum, tally| FrameTally {
                presented: sum.presented.saturating_add(tally.presented),
                missed: sum.missed.saturating_add(tally.missed),
            })
    }

    /// Stops or resumes frames (a VT switch, a suspend).
    pub fn flow(&mut self, flow: Flow) {
        self.flow = flow;
    }

    /// The scanout of `name`.
    pub fn get(&self, name: &OutputName) -> Option<&Scanout> {
        self.outputs.get(name)
    }

    /// The scanout of `name`, mutably.
    pub fn get_mut(&mut self, name: &OutputName) -> Option<&mut Scanout> {
        self.outputs.get_mut(name)
    }

    /// Queues a frame of `source` for `name` at `now`; none while held, unlit or a flip is
    /// pending.
    pub fn present(
        &mut self,
        name: &OutputName,
        source: &dyn FrameSource,
        now: Mono,
    ) -> Result<PresentOutcome, BackendError> {
        match (self.flow, self.outputs.get_mut(name)) {
            (Flow::Running, Some(scanout)) => scanout.present(source, now),
            _ => Ok(PresentOutcome::Waiting),
        }
    }

    /// The seat came back: every scanout forgets the screen.
    pub fn reset(&mut self) {
        self.outputs.values_mut().for_each(Scanout::reset);
    }

    /// Whether the display shows the session at `now`, over the outputs that scan out: see
    /// [`ScanoutSet::health_of`].
    pub fn health(&self, now: Mono) -> Health {
        self.health_of(now, &self.names())
    }

    /// Whether the display shows the session at `now`, given the outputs that should be lit
    /// (`enabled`): frozen when there is one and every one of them has either asked for frames
    /// for too long and shown none, or has no scanout at all (a re-light that failed left it
    /// without; nothing will ever show on it, CASB-05).
    pub fn health_of(&self, now: Mono, enabled: &[OutputName]) -> Health {
        let starved = |name: &OutputName| {
            self.outputs
                .get(name)
                .is_none_or(|scanout| scanout.starved(now))
        };
        match !enabled.is_empty() && enabled.iter().all(starved) {
            true => Health::Frozen,
            false => Health::Alive,
        }
    }

    /// The outputs that scan out now.
    pub fn names(&self) -> Vec<OutputName> {
        self.outputs.keys().cloned().collect()
    }

    /// Whether frames flow.
    pub fn flowing(&self) -> Flow {
        self.flow
    }
}
