//! The session lock (ext-session-lock). Once locked the session stays covered until the
//! locker unlocks: a locker that dies leaves the cover up, and only the shell (or the client that
//! held the lock) may take the lock over.

use super::Model;
use super::moved::Moved;
use crate::effect::Effect;
use crate::ids::{ClientKey, OutputKey};
use crate::lock::{LockEvent, LockHolder, LockReply, LockState, LockSurfaces};
use crate::time::{Due, TimerKey};
use std::time::Duration;

/// How long a locker has to make its first lock surface before the session is locked without
/// one (ext-session-lock: the compositor "must impose a reasonable time limit" if it waits).
pub const LOCK_WAIT: Duration = Duration::from_secs(1);

impl Model {
    /// The lock moved. The lock marker follows: written when the shell's lock is held, removed
    /// when it is not.
    pub(super) fn lock_event(self, event: LockEvent) -> Moved {
        let before = self.marker();
        let moved = self.lock_moved(event);
        match (before, moved.model.marker()) {
            (false, true) => moved.with(Effect::Lock(LockReply::Remember)),
            (true, false) => moved.with(Effect::Lock(LockReply::Forget)),
            (false, false) | (true, true) => moved,
        }
    }

    /// Whether the lock marker should exist: the shell holds the lock (`locked` was sent, or
    /// it is held with no locker). A lock another client holds is not remembered: after a
    /// restart only the shell can take a held lock over, so a marker for any other locker could
    /// leave the person with a cover nobody comes to lift.
    fn marker(&self) -> bool {
        matches!(
            &self.lock,
            LockState::Locked {
                holder: LockHolder::Shell(_),
                ..
            } | LockState::LockedWithoutSurface {
                holder: LockHolder::Shell(_),
            }
        )
    }

    fn lock_moved(self, event: LockEvent) -> Moved {
        match (self.lock.clone(), event) {
            (LockState::Unlocked, LockEvent::Restored) => {
                self.with_lock(LockState::LockedWithoutSurface {
                    holder: LockHolder::Shell(None),
                })
            }
            (_, LockEvent::Restored) => Moved::of(self),
            (LockState::Unlocked, LockEvent::Requested { client }) => self
                .begin_lock(client)
                .then(|model| model.leave_overview(None)),
            (
                LockState::LockedWithoutSurface { holder: before },
                LockEvent::Requested { client },
            ) if self.may_take_over(before, client) => {
                let holder = self.holder_of(client);
                self.with_lock(LockState::Locked {
                    holder,
                    surfaces: LockSurfaces::Pending,
                })
                .with(Effect::Lock(LockReply::SendLocked))
            }
            (_, LockEvent::Requested { .. }) => {
                Moved::of(self).with(Effect::Lock(LockReply::Finished))
            }
            (LockState::Unlocked, _) => Moved::of(self),
            (state, LockEvent::SurfaceCommitted { output }) => self.commit_surface(state, output),
            (state, LockEvent::SurfaceGone { output }) => self.lose_surface(state, &output),
            (state, LockEvent::Unlocked) => self.end_lock(&state),
            (state, LockEvent::LockerGone) => self.lose_locker(&state),
        }
    }

    /// `f` on the model while the session is open. A locked session (or one being locked)
    /// ignores what clients ask of windows, Spaces and the desktop: they find it as they left
    /// it when the person unlocks.
    pub(super) fn unless_locked(self, f: impl FnOnce(Model) -> Moved) -> Moved {
        match self.lock {
            LockState::Unlocked => f(self),
            _ => Moved::of(self),
        }
    }

    /// Whether `client` may take over a lock whose locker is gone: the shell may (it is the
    /// person's own locker), and so may the client that held the lock. Any other client is
    /// refused, so a program that waits for the locker to die cannot unlock the session.
    fn may_take_over(&self, holder: LockHolder, client: ClientKey) -> bool {
        self.is_shell(client) || holder.client() == Some(client)
    }

    /// `client` as the holder of a lock it asked for.
    fn holder_of(&self, client: ClientKey) -> LockHolder {
        match self.is_shell(client) {
            true => LockHolder::Shell(Some(client)),
            false => LockHolder::Client(client),
        }
    }

    /// The outputs the lock covers with its own cover, because no locker surface does.
    fn covers_for(&self, state: &LockState) -> Vec<OutputKey> {
        match state {
            LockState::Locked {
                surfaces: LockSurfaces::Covering(covered),
                ..
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
            | LockState::LockedWithoutSurface { .. } => self.outputs.0.keys().cloned().collect(),
        }
    }

    fn with_lock(self, lock: LockState) -> Moved {
        Moved::of(Model { lock, ..self })
    }

    /// A locker asked: every output goes under the cover. With no output there is nothing to
    /// wait for, and the session is locked at once.
    fn begin_lock(self, client: ClientKey) -> Moved {
        let outputs: Vec<OutputKey> = self.outputs.0.keys().cloned().collect();
        let relisted = self.relisted();
        let holder = self.holder_of(client);
        if outputs.is_empty() {
            return self
                .with_lock(LockState::Locked {
                    holder,
                    surfaces: LockSurfaces::Pending,
                })
                .with(Effect::Lock(LockReply::SendLocked))
                .with_all(relisted);
        }
        let since = self.now;
        let wait = Due {
            at: since.after(LOCK_WAIT),
            key: TimerKey::LockWait,
        };
        self.with_lock(LockState::Locking { since, holder })
            .with_all(outputs.into_iter().map(show_cover))
            .with(Effect::ArmTimer(wait))
            .with_all(relisted)
    }

    /// Every window listed and published again: window-list clients are told no titles while
    /// the session is locked, and the real ones when it opens.
    fn relisted(&self) -> Vec<Effect> {
        self.windows
            .keys()
            .flat_map(|id| [Effect::ListWindow(*id), Effect::PublishToplevel(*id)])
            .collect()
    }

    /// The locker has had its time to make a lock surface: every output is under the cover
    /// already, so the session is locked without one. A surface that comes later is taken in.
    /// (A wait that already ended with a surface, an unlock or a dead locker changes nothing.)
    pub(super) fn lock_wait_over(self) -> Moved {
        match self.lock {
            LockState::Locking { holder, .. } => self
                .with_lock(LockState::Locked {
                    holder,
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
        let (holder, covered, announce) = match state {
            LockState::Locking { holder, .. } => (
                holder,
                vec![output.clone()],
                vec![
                    Effect::Lock(LockReply::SendLocked),
                    Effect::CancelTimer(TimerKey::LockWait),
                ],
            ),
            LockState::Locked {
                holder,
                surfaces: LockSurfaces::Covering(covered),
            } => match covered.contains(&output) {
                true => return Moved::of(self),
                false => (holder, with_output(covered, &output), Vec::new()),
            },
            LockState::Locked {
                holder,
                surfaces: LockSurfaces::Pending,
            } => (holder, vec![output.clone()], Vec::new()),
            LockState::Unlocked | LockState::LockedWithoutSurface { .. } => {
                return Moved::of(self);
            }
        };
        self.with_lock(LockState::Locked {
            holder,
            surfaces: LockSurfaces::Covering(covered),
        })
        .with(Effect::Lock(LockReply::HideCover(output)))
        .with_all(announce)
    }

    /// The locker's surface on `output` is gone while the lock holds: the output is covered by
    /// the cover again, so the desktop never shows through, and the locker is back to having
    /// covered the rest. (Before `locked` nothing has been hidden for it yet.)
    fn lose_surface(self, state: LockState, output: &OutputKey) -> Moved {
        let LockState::Locked {
            holder,
            surfaces: LockSurfaces::Covering(covered),
        } = state
        else {
            return Moved::of(self);
        };
        if !covered.contains(output) {
            return Moved::of(self);
        }
        let rest: Vec<OutputKey> = covered.into_iter().filter(|held| held != output).collect();
        let surfaces = match rest.is_empty() {
            true => LockSurfaces::Pending,
            false => LockSurfaces::Covering(rest),
        };
        self.with_lock(LockState::Locked { holder, surfaces })
            .with(show_cover(output.clone()))
    }

    /// The locker unlocked: the covers come down and the keyboard goes back to the desktop.
    fn end_lock(self, state: &LockState) -> Moved {
        let covers = self.covers_for(state);
        let relisted = self.relisted();
        self.with_lock(LockState::Unlocked)
            .with_all(covers.into_iter().map(hide_cover))
            .with_all(relisted)
    }

    /// The locker is gone. Before `locked` was sent nothing was ever locked, so the session
    /// opens again (a locker that crashes at start-up must not lock the person out). After it,
    /// the session stays locked, and each output its surface covered shows the cover instead.
    fn lose_locker(self, state: &LockState) -> Moved {
        match state {
            LockState::Unlocked | LockState::LockedWithoutSurface { .. } => Moved::of(self),
            LockState::Locking { .. } => self.abandon_lock(state),
            LockState::Locked { holder, surfaces } => {
                let uncovered = match surfaces {
                    LockSurfaces::Covering(covered) => covered.clone(),
                    LockSurfaces::Pending => Vec::new(),
                };
                self.with_lock(LockState::LockedWithoutSurface { holder: *holder })
                    .with_all(uncovered.into_iter().map(show_cover))
            }
        }
    }

    /// The locker died before it was told `locked`: the covers come down, the wait for its
    /// first surface ends, and the session is open.
    fn abandon_lock(self, state: &LockState) -> Moved {
        let covers = self.covers_for(state);
        let relisted = self.relisted();
        self.with_lock(LockState::Unlocked)
            .with(Effect::CancelTimer(TimerKey::LockWait))
            .with_all(covers.into_iter().map(hide_cover))
            .with_all(relisted)
    }

    /// An output left the lock's coverage.
    pub(super) fn lock_without_output(self, output: &OutputKey) -> Model {
        let lock = match &self.lock {
            LockState::Locked {
                holder,
                surfaces: LockSurfaces::Covering(covered),
            } => {
                let rest: Vec<OutputKey> = covered
                    .iter()
                    .filter(|held| *held != output)
                    .cloned()
                    .collect();
                LockState::Locked {
                    holder: *holder,
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
