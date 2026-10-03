//! The timer book: what is armed, keyed by what it is for, in due order with ties broken by
//! the order they were armed.

use super::Mono;
use crate::app::AppKey;
use crate::ids::{ClientKey, DeviceId, LeaseId, OutputKey};
use casement_protocols::vocab::Corner;
use std::collections::BTreeMap;

/// One `ext_idle_notification_v1` a client asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct IdleNotifierId(pub u64);

/// One trusted surface (`quire_trusted_surface_v1`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TrustedKey(pub u64);

/// What a timer is for. A key is armed at most once: arming it again moves it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TimerKey {
    /// The next repeat of the key held on this keyboard.
    KeyRepeat(DeviceId),
    /// A ping to this client went unanswered for the not-responding interval.
    Ping(ClientKey),
    /// A client's idle notification is due.
    IdleNotify(ClientKey, IdleNotifierId),
    /// An app asked to quit has had its grace period.
    QuitGrace(AppKey),
    /// An output has waited too long for a frame.
    FrameWatch(OutputKey),
    /// The app switcher's appearance delay.
    SwitcherDelay,
    /// The pointer has rested in a hot corner long enough.
    HotCorner(Corner),
    /// A Space change animation on this output finishes.
    SpaceFinish(OutputKey),
    /// A trusted surface becomes able to take an answer.
    TrustedArm(TrustedKey),
    /// An agent lease's yield period ends.
    LeaseYield(LeaseId),
}

/// A timer: when, and what for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Due {
    /// When it fires.
    pub at: Mono,
    /// What it is for.
    pub key: TimerKey,
}

/// One armed timer's instant and its place in arming order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Armed {
    at: Mono,
    issue: u64,
}

/// Every armed timer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimerBook {
    armed: BTreeMap<TimerKey, Armed>,
    issued: u64,
}

impl TimerBook {
    /// No timers.
    pub fn empty() -> TimerBook {
        TimerBook {
            armed: BTreeMap::new(),
            issued: 0,
        }
    }

    /// The book with `due` armed (replacing any timer with the same key, which then counts as
    /// armed last).
    pub fn arm(self, due: Due) -> TimerBook {
        let TimerBook { mut armed, issued } = self;
        armed.insert(
            due.key,
            Armed {
                at: due.at,
                issue: issued,
            },
        );
        TimerBook {
            armed,
            issued: issued.saturating_add(1),
        }
    }

    /// The book without the timer for `key`.
    pub fn cancel(self, key: &TimerKey) -> TimerBook {
        let TimerBook { mut armed, issued } = self;
        armed.remove(key);
        TimerBook { armed, issued }
    }

    /// The timer that fires first (earliest instant, then earliest armed).
    pub fn next(&self) -> Option<Due> {
        self.in_order().into_iter().next()
    }

    /// The earliest instant a timer is due strictly after `t`.
    pub fn next_after(&self, t: Mono) -> Option<Mono> {
        self.armed
            .values()
            .map(|armed| armed.at)
            .filter(|at| *at > t)
            .min()
    }

    /// The book without every timer due at or before `t`, and their keys in firing order.
    pub fn fire_until(self, t: Mono) -> (TimerBook, Vec<TimerKey>) {
        let fired: Vec<TimerKey> = self
            .in_order()
            .into_iter()
            .take_while(|due| due.at <= t)
            .map(|due| due.key)
            .collect();
        let book = fired.iter().fold(self, |book, key| book.cancel(key));
        (book, fired)
    }

    /// Every armed timer, in firing order.
    fn in_order(&self) -> Vec<Due> {
        let mut armed: Vec<(&TimerKey, &Armed)> = self.armed.iter().collect();
        armed.sort_by_key(|(_, armed)| (armed.at, armed.issue));
        armed
            .into_iter()
            .map(|(key, armed)| Due {
                at: armed.at,
                key: key.clone(),
            })
            .collect()
    }
}
