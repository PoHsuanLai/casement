//! The idle machine: which notifications watch, how long each has been quiet, and what to arm,
//! cancel or tell a client when time, activity or an inhibitor moves. Pure: every instant is an
//! argument (`Mono`), so a test steps it at hand-picked instants and the fixture's virtual
//! clock decides when the compositor does.
//!
//! Timers are the session's (`TimerKey::IdleNotify`), never smithay's wall-clock ones (A36).
//! Activity is lazy: it only moves a watch's quiet-since mark, and a timer that fires early
//! re-arms for what is left, so a stream of pointer motion arms nothing and touches nothing: the
//! book keeps one "last activity" instant, never a mark per watch, and the set of watches that
//! are idle, which are the only ones activity has anything to tell (NEW-CAS-2).

use casement_core::time::{Due, IdleNotifierId, Mono, TimerKey};
use casement_core::{ClientKey, Effect};
use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

/// Where one watch is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    /// Counting quiet time since this instant (its timer is armed, or about to be re-armed).
    Quiet { since: Mono },
    /// Told `idled`; waiting for activity.
    Idle,
    /// An inhibitor holds it: no timer, nothing told.
    Held,
}

/// One notification a client asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Watch {
    client: ClientKey,
    timeout: Duration,
    /// Whether inhibitors keep it from going idle: `get_idle_notification` does,
    /// `get_input_idle_notification` (version 2) counts input only.
    honours_inhibitors: bool,
    phase: Phase,
}

impl Watch {
    fn key(&self, id: IdleNotifierId) -> TimerKey {
        TimerKey::IdleNotify(self.client, id)
    }

    fn arm(&self, id: IdleNotifierId, since: Mono) -> Effect {
        Effect::ArmTimer(Due {
            at: since.after(self.timeout),
            key: self.key(id),
        })
    }
}

/// Something a client is told.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signal {
    /// `ext_idle_notification_v1.idled`.
    Idled(IdleNotifierId),
    /// `ext_idle_notification_v1.resumed`.
    Resumed(IdleNotifierId),
}

/// What a transition asks for: timers for `exec`, events for the wire, in order.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Moved {
    /// Timers to arm or cancel.
    pub effects: Vec<Effect>,
    /// What to tell clients.
    pub signals: Vec<Signal>,
}

impl Moved {
    fn effect(mut self, effect: Effect) -> Moved {
        self.effects.push(effect);
        self
    }

    fn signal(mut self, signal: Signal) -> Moved {
        self.signals.push(signal);
        self
    }
}

/// Every watch, whether an inhibitor holds idleness off, and the next notifier id.
///
/// Activity is one instant (`active_at`): a quiet watch has been quiet since the later of its
/// own mark and that instant. The watches that are idle are kept apart (`idle`), since they
/// are the only ones activity has to tell. Both make an input event cost nothing when nothing
/// is idle, however many watches there are.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IdleBook {
    watches: BTreeMap<IdleNotifierId, Watch>,
    /// The watches in [`Phase::Idle`].
    idle: BTreeSet<IdleNotifierId>,
    /// How many watches each client holds.
    held: BTreeMap<ClientKey, usize>,
    /// The latest input of any origin.
    active_at: Option<Mono>,
    /// Whether a visible surface holds idleness off (`zwp_idle_inhibitor_v1`).
    inhibited: bool,
    next: u64,
}

impl IdleBook {
    /// A client asked to be told after `timeout` of quiet at `now`. The id is minted here.
    pub fn watch(
        self,
        client: ClientKey,
        timeout: Duration,
        honours_inhibitors: bool,
        now: Mono,
    ) -> (IdleBook, IdleNotifierId, Moved) {
        let id = IdleNotifierId(self.next);
        let held = honours_inhibitors && self.inhibited;
        let watch = Watch {
            client,
            timeout,
            honours_inhibitors,
            phase: if held {
                Phase::Held
            } else {
                Phase::Quiet { since: now }
            },
        };
        let moved = match watch.phase {
            Phase::Quiet { since } => Moved::default().effect(watch.arm(id, since)),
            Phase::Idle | Phase::Held => Moved::default(),
        };
        let mut watches = self.watches;
        watches.insert(id, watch);
        let mut held = self.held;
        *held.entry(client).or_insert(0) += 1;
        let book = IdleBook {
            watches,
            held,
            next: self.next.saturating_add(1),
            ..self
        };
        (book, id, moved)
    }

    /// Input of any origin arrived at `now`: every quiet watch counts from here (one instant
    /// moves, no watch is visited), every idle one is told `resumed` and counts again (its
    /// timer was spent, so it is re-armed).
    pub fn activity(self, now: Mono) -> (IdleBook, Moved) {
        let IdleBook {
            mut watches,
            idle,
            held,
            inhibited,
            next,
            ..
        } = self;
        let moved = idle
            .iter()
            .fold(Moved::default(), |moved, id| match watches.get_mut(id) {
                Some(watch) => {
                    watch.phase = Phase::Quiet { since: now };
                    moved
                        .signal(Signal::Resumed(*id))
                        .effect(watch.arm(*id, now))
                }
                None => moved,
            });
        let book = IdleBook {
            watches,
            idle: BTreeSet::new(),
            held,
            active_at: Some(now),
            inhibited,
            next,
        };
        (book, moved)
    }

    /// The timer of watch `id` of `client` fired at `now`. If the watch has been quiet for its
    /// whole timeout it is told `idled`; if activity moved its mark since the timer was armed,
    /// it is re-armed for what is left. A timer for a watch that is gone, held or already idle
    /// does nothing.
    pub fn fire(self, client: ClientKey, id: IdleNotifierId, now: Mono) -> (IdleBook, Moved) {
        let Some(watch) = self
            .watches
            .get(&id)
            .copied()
            .filter(|w| w.client == client)
        else {
            return (self, Moved::default());
        };
        let Phase::Quiet { since } = watch.phase else {
            return (self, Moved::default());
        };
        // Quiet since its own mark or the latest input, whichever is later.
        let since = self.active_at.map_or(since, |active| since.max(active));
        let due = since.after(watch.timeout);
        let (phase, moved) = if now >= due {
            (Phase::Idle, Moved::default().signal(Signal::Idled(id)))
        } else {
            (
                Phase::Quiet { since },
                Moved::default().effect(watch.arm(id, since)),
            )
        };
        let mut idle = self.idle;
        if phase == Phase::Idle {
            idle.insert(id);
        }
        let mut watches = self.watches;
        watches.insert(id, Watch { phase, ..watch });
        (
            IdleBook {
                watches,
                idle,
                ..self
            },
            moved,
        )
    }

    /// An inhibitor appeared or the last one went at `now`. Raised: watches that obey it are
    /// told `resumed` if idle, and stop counting. Lowered: they count again from `now`.
    pub fn inhibit(self, inhibited: bool, now: Mono) -> (IdleBook, Moved) {
        if inhibited == self.inhibited {
            return (self, Moved::default());
        }
        let IdleBook {
            watches,
            mut idle,
            held,
            active_at,
            next,
            ..
        } = self;
        let (watches, moved) = watches.into_iter().fold(
            (BTreeMap::new(), Moved::default()),
            |(mut kept, moved), (id, watch)| {
                let (phase, moved) = match (watch.honours_inhibitors, inhibited, watch.phase) {
                    (false, _, phase) => (phase, moved),
                    (true, true, Phase::Idle) => {
                        idle.remove(&id);
                        (Phase::Held, moved.signal(Signal::Resumed(id)))
                    }
                    (true, true, Phase::Quiet { .. }) => (
                        Phase::Held,
                        moved.effect(Effect::CancelTimer(watch.key(id))),
                    ),
                    (true, false, Phase::Held) => (
                        Phase::Quiet { since: now },
                        moved.effect(watch.arm(id, now)),
                    ),
                    (true, _, phase) => (phase, moved),
                };
                kept.insert(id, Watch { phase, ..watch });
                (kept, moved)
            },
        );
        (
            IdleBook {
                watches,
                idle,
                held,
                active_at,
                inhibited,
                next,
            },
            moved,
        )
    }

    /// The watch `id` is gone (its object was destroyed or its client left): its timer is
    /// cancelled.
    pub fn forget(self, id: IdleNotifierId) -> (IdleBook, Moved) {
        let mut watches = self.watches;
        let mut idle = self.idle;
        let mut held = self.held;
        idle.remove(&id);
        let moved = watches.remove(&id).map_or_else(Moved::default, |watch| {
            if let Some(count) = held.get_mut(&watch.client) {
                *count = count.saturating_sub(1);
                if *count == 0 {
                    held.remove(&watch.client);
                }
            }
            Moved::default().effect(Effect::CancelTimer(watch.key(id)))
        });
        (
            IdleBook {
                watches,
                idle,
                held,
                ..self
            },
            moved,
        )
    }

    /// Whether an inhibitor holds idleness off.
    pub fn inhibited(&self) -> bool {
        self.inhibited
    }

    /// How many watches `client` holds (the per-client cap counts these).
    pub fn held_by(&self, client: ClientKey) -> usize {
        self.held.get(&client).copied().unwrap_or(0)
    }

    /// How many watches there are.
    pub fn len(&self) -> usize {
        self.watches.len()
    }

    /// Whether there are none.
    pub fn is_empty(&self) -> bool {
        self.watches.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CLIENT: ClientKey = ClientKey(1);

    fn at(ms: u64) -> Mono {
        Mono(Duration::from_millis(ms))
    }

    fn arm(id: IdleNotifierId, ms: u64) -> Effect {
        Effect::ArmTimer(Due {
            at: at(ms),
            key: TimerKey::IdleNotify(CLIENT, id),
        })
    }

    fn cancel(id: IdleNotifierId) -> Effect {
        Effect::CancelTimer(TimerKey::IdleNotify(CLIENT, id))
    }

    fn watching(timeout_ms: u64, honours_inhibitors: bool, now: u64) -> (IdleBook, IdleNotifierId) {
        let (book, id, _) = IdleBook::default().watch(
            CLIENT,
            Duration::from_millis(timeout_ms),
            honours_inhibitors,
            at(now),
        );
        (book, id)
    }

    #[test]
    fn a_watch_arms_its_timeout_and_goes_idle_exactly_then() {
        let (book, id, moved) =
            IdleBook::default().watch(CLIENT, Duration::from_secs(30), true, at(5));
        assert_eq!(moved.effects, vec![arm(id, 30_005)]);
        assert!(moved.signals.is_empty());
        let (_, early) = book.clone().fire(CLIENT, id, at(30_004));
        assert!(early.signals.is_empty(), "one tick early says nothing");
        let (_, due) = book.fire(CLIENT, id, at(30_005));
        assert_eq!(due.signals, vec![Signal::Idled(id)]);
    }

    #[test]
    fn activity_moves_the_mark_and_an_early_timer_re_arms_for_the_rest() {
        let (book, id) = watching(30_000, true, 0);
        let (book, moved) = book.activity(at(10_000));
        assert_eq!(moved, Moved::default(), "motion arms nothing");
        let (book, moved) = book.fire(CLIENT, id, at(30_000));
        assert_eq!(moved.effects, vec![arm(id, 40_000)]);
        assert!(moved.signals.is_empty());
        let (_, moved) = book.fire(CLIENT, id, at(40_000));
        assert_eq!(moved.signals, vec![Signal::Idled(id)]);
    }

    #[test]
    fn activity_after_idled_resumes_and_re_arms() {
        let (book, id) = watching(1_000, true, 0);
        let (book, _) = book.fire(CLIENT, id, at(1_000));
        let (book, moved) = book.activity(at(2_500));
        assert_eq!(moved.signals, vec![Signal::Resumed(id)]);
        assert_eq!(moved.effects, vec![arm(id, 3_500)]);
        let (_, moved) = book.fire(CLIENT, id, at(3_500));
        assert_eq!(moved.signals, vec![Signal::Idled(id)]);
    }

    #[test]
    fn a_second_timer_after_idled_says_nothing() {
        let (book, id) = watching(1_000, true, 0);
        let (book, _) = book.fire(CLIENT, id, at(1_000));
        let (_, moved) = book.fire(CLIENT, id, at(1_000));
        assert_eq!(moved, Moved::default());
    }

    #[test]
    fn an_inhibitor_holds_idled_off_and_its_release_restarts_the_count() {
        let (book, id) = watching(1_000, true, 0);
        let (book, moved) = book.inhibit(true, at(500));
        assert_eq!(moved.effects, vec![cancel(id)]);
        let (book, moved) = book.fire(CLIENT, id, at(1_000));
        assert_eq!(
            moved,
            Moved::default(),
            "a held watch ignores a stale timer"
        );
        let (book, moved) = book.inhibit(false, at(4_000));
        assert_eq!(moved.effects, vec![arm(id, 5_000)]);
        let (_, moved) = book.fire(CLIENT, id, at(5_000));
        assert_eq!(moved.signals, vec![Signal::Idled(id)]);
    }

    #[test]
    fn an_inhibitor_raised_after_idled_resumes_the_client() {
        let (book, id) = watching(1_000, true, 0);
        let (book, _) = book.fire(CLIENT, id, at(1_000));
        let (book, moved) = book.inhibit(true, at(1_500));
        assert_eq!(moved.signals, vec![Signal::Resumed(id)]);
        let (_, moved) = book.activity(at(2_000));
        assert_eq!(
            moved,
            Moved::default(),
            "a held watch stays held through activity"
        );
    }

    #[test]
    fn a_watch_made_while_inhibited_starts_held() {
        let (book, _) = watching(1_000, true, 0);
        let (book, _) = book.inhibit(true, at(10));
        let (_, _, moved) = book.watch(CLIENT, Duration::from_secs(1), true, at(20));
        assert_eq!(moved, Moved::default());
    }

    #[test]
    fn an_input_idle_watch_ignores_inhibitors() {
        let (book, id) = watching(1_000, false, 0);
        let (book, moved) = book.inhibit(true, at(100));
        assert_eq!(moved, Moved::default());
        let (_, moved) = book.fire(CLIENT, id, at(1_000));
        assert_eq!(moved.signals, vec![Signal::Idled(id)]);
    }

    #[test]
    fn a_destroyed_watch_cancels_its_timer_and_forgets_stale_fires() {
        let (book, id) = watching(1_000, true, 0);
        let (book, moved) = book.forget(id);
        assert_eq!(moved.effects, vec![cancel(id)]);
        assert!(book.is_empty());
        let (_, moved) = book.fire(CLIENT, id, at(1_000));
        assert_eq!(moved, Moved::default());
    }

    #[test]
    fn a_timer_of_another_client_does_nothing() {
        let (book, id) = watching(1_000, true, 0);
        let (_, moved) = book.fire(ClientKey(2), id, at(1_000));
        assert_eq!(moved, Moved::default());
    }

    /// Many quiet watches and a stream of input: activity visits none of them, and each still
    /// goes idle exactly one timeout after the last input (the later of its mark and the input).
    #[test]
    fn many_quiet_watches_stay_lazy_under_input_and_still_go_idle_after_the_last_one() {
        let mut book = IdleBook::default();
        let mut ids = Vec::new();
        for _ in 0..2_000 {
            let (next, id, _) = book.watch(CLIENT, Duration::from_millis(1_000), true, at(0));
            book = next;
            ids.push(id);
        }
        assert_eq!(book.held_by(CLIENT), 2_000);
        for ms in (100..900).step_by(100) {
            let (next, moved) = book.activity(at(ms));
            book = next;
            assert_eq!(
                moved,
                Moved::default(),
                "input at {ms} tells and arms nothing"
            );
        }
        let first = ids[0];
        let (book, early) = book.fire(CLIENT, first, at(1_000));
        assert!(early.signals.is_empty(), "input at 800 moved the mark");
        assert_eq!(early.effects, vec![arm(first, 1_800)]);
        let (_, due) = book.fire(CLIENT, first, at(1_800));
        assert_eq!(due.signals, vec![Signal::Idled(first)]);
    }

    #[test]
    fn the_per_client_count_follows_watches_made_and_destroyed() {
        let (book, first) = watching(1_000, true, 0);
        let (book, second, _) = book.watch(CLIENT, Duration::from_secs(1), true, at(0));
        let (book, _, _) = book.watch(ClientKey(2), Duration::from_secs(1), true, at(0));
        assert_eq!((book.held_by(CLIENT), book.held_by(ClientKey(2))), (2, 1));
        let (book, _) = book.forget(first);
        let (book, _) = book.forget(second);
        let (book, _) = book.forget(second);
        assert_eq!((book.held_by(CLIENT), book.held_by(ClientKey(2))), (0, 1));
    }

    #[test]
    fn ids_are_minted_in_order() {
        let (book, first) = watching(1_000, true, 0);
        let (_, second, _) = book.watch(CLIENT, Duration::from_secs(1), true, at(0));
        assert_eq!((first, second), (IdleNotifierId(0), IdleNotifierId(1)));
    }
}
