//! The session lock (ext-session-lock). Once locked the session stays covered until the
//! locker unlocks: a locker that dies leaves the cover up, and another may take the lock over.

use super::Model;
use super::moved::Moved;
use crate::effect::Effect;
use crate::ids::OutputKey;
use crate::lock::{LockEvent, LockReply, LockState, LockSurfaces};
use crate::time::{Due, TimerKey};
use std::time::Duration;

/// How long a locker has to make its first lock surface before the session is locked without
/// one (ext-session-lock: the compositor "must impose a reasonable time limit" if it waits).
pub const LOCK_WAIT: Duration = Duration::from_secs(1);

impl Model {
    /// The lock moved.
    pub(super) fn lock_event(self, event: LockEvent) -> Moved {
        match (self.lock.clone(), event) {
            (LockState::Unlocked, LockEvent::Requested { .. }) => self.begin_lock(),
            (LockState::LockedWithoutSurface, LockEvent::Requested { .. }) => self
                .with_lock(LockState::Locked {
                    surfaces: LockSurfaces::Pending,
                })
                .with(Effect::Lock(LockReply::SendLocked)),
            (_, LockEvent::Requested { .. }) => {
                Moved::of(self).with(Effect::Lock(LockReply::Finished))
            }
            (LockState::Unlocked, _) => Moved::of(self),
            (state, LockEvent::SurfaceCommitted { output }) => self.commit_surface(state, output),
            (state, LockEvent::Unlocked) => self.end_lock(&state),
            (state, LockEvent::LockerGone) => self.lose_locker(&state),
        }
    }

    /// The outputs the lock covers with its own cover, because no locker surface does.
    fn covers_for(&self, state: &LockState) -> Vec<OutputKey> {
        match state {
            LockState::Locked {
                surfaces: LockSurfaces::Covering(covered),
            } => self
                .outputs
                .0
                .keys()
                .filter(|output| !covered.contains(output))
                .cloned()
                .collect(),
            LockState::Unlocked => Vec::new(),
            LockState::Locking { .. }
            | LockState::Locked { .. }
            | LockState::LockedWithoutSurface => self.outputs.0.keys().cloned().collect(),
        }
    }

    fn with_lock(self, lock: LockState) -> Moved {
        Moved::of(Model { lock, ..self })
    }

    /// A locker asked: every output goes under the cover. With no output there is nothing to
    /// wait for, and the session is locked at once.
    fn begin_lock(self) -> Moved {
        let outputs: Vec<OutputKey> = self.outputs.0.keys().cloned().collect();
        if outputs.is_empty() {
            return self
                .with_lock(LockState::Locked {
                    surfaces: LockSurfaces::Pending,
                })
                .with(Effect::Lock(LockReply::SendLocked));
        }
        let since = self.now;
        let wait = Due {
            at: since.after(LOCK_WAIT),
            key: TimerKey::LockWait,
        };
        self.with_lock(LockState::Locking { since })
            .with_all(outputs.into_iter().map(show_cover))
            .with(Effect::ArmTimer(wait))
    }

    /// The locker has had its time to make a lock surface: every output is under the cover
    /// already, so the session is locked without one. A surface that comes later is taken in.
    /// (A wait that already ended with a surface, an unlock or a dead locker changes nothing.)
    pub(super) fn lock_wait_over(self) -> Moved {
        match self.lock {
            LockState::Locking { .. } => self
                .with_lock(LockState::Locked {
                    surfaces: LockSurfaces::Pending,
                })
                .with(Effect::Lock(LockReply::SendLocked)),
            _ => Moved::of(self),
        }
    }

    /// The locker's surface on `output` committed: the cover there gives way to it. The first
    /// one means every output is covered, by a surface or by the cover, so `locked` is sent.
    fn commit_surface(self, state: LockState, output: OutputKey) -> Moved {
        if !self.outputs.0.contains_key(&output) {
            return Moved::of(self);
        }
        let (covered, announce) = match state {
            LockState::Locking { .. } => (
                vec![output.clone()],
                vec![
                    Effect::Lock(LockReply::SendLocked),
                    Effect::CancelTimer(TimerKey::LockWait),
                ],
            ),
            LockState::Locked {
                surfaces: LockSurfaces::Covering(covered),
            } => match covered.contains(&output) {
                true => return Moved::of(self),
                false => (with_output(covered, &output), Vec::new()),
            },
            LockState::Locked {
                surfaces: LockSurfaces::Pending,
            } => (vec![output.clone()], Vec::new()),
            LockState::Unlocked | LockState::LockedWithoutSurface => return Moved::of(self),
        };
        self.with_lock(LockState::Locked {
            surfaces: LockSurfaces::Covering(covered),
        })
        .with(Effect::Lock(LockReply::HideCover(output)))
        .with_all(announce)
    }

    /// The locker unlocked: the covers come down and the keyboard goes back to the desktop.
    fn end_lock(self, state: &LockState) -> Moved {
        let covers = self.covers_for(state);
        self.with_lock(LockState::Unlocked)
            .with_all(covers.into_iter().map(hide_cover))
    }

    /// The locker is gone: the session stays locked, and each output its surface covered
    /// shows the cover instead.
    fn lose_locker(self, state: &LockState) -> Moved {
        let uncovered = match state {
            LockState::Locked {
                surfaces: LockSurfaces::Covering(covered),
            } => covered.clone(),
            _ => Vec::new(),
        };
        match state {
            LockState::LockedWithoutSurface | LockState::Unlocked => Moved::of(self),
            _ => self
                .with_lock(LockState::LockedWithoutSurface)
                .with_all(uncovered.into_iter().map(show_cover)),
        }
    }

    /// An output left the lock's coverage.
    pub(super) fn lock_without_output(self, output: &OutputKey) -> Model {
        let lock = match &self.lock {
            LockState::Locked {
                surfaces: LockSurfaces::Covering(covered),
            } => {
                let rest: Vec<OutputKey> = covered
                    .iter()
                    .filter(|held| *held != output)
                    .cloned()
                    .collect();
                LockState::Locked {
                    surfaces: match rest.is_empty() {
                        true => LockSurfaces::Pending,
                        false => LockSurfaces::Covering(rest),
                    },
                }
            }
            other => other.clone(),
        };
        Model { lock, ..self }
    }

    /// The cover for a new output while the session is locked.
    pub(super) fn cover_for_new(&self, output: &OutputKey) -> Option<Effect> {
        match self.lock {
            LockState::Unlocked => None,
            _ => Some(show_cover(output.clone())),
        }
    }
}

fn show_cover(output: OutputKey) -> Effect {
    Effect::Lock(LockReply::ShowCover(output))
}

fn hide_cover(output: OutputKey) -> Effect {
    Effect::Lock(LockReply::HideCover(output))
}

/// `covered` with `output` among them, in key order, once.
fn with_output(covered: Vec<OutputKey>, output: &OutputKey) -> Vec<OutputKey> {
    let mut all = covered;
    all.push(output.clone());
    all.sort();
    all.dedup();
    all
}
