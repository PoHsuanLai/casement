//! How many `org.quire.Compositor1` calls casement takes: a rate per caller, and a queue of a
//! few into the event loop. A caller that asks too often, or a loop that is behind, is told so
//! at once; nothing waits without bound (CASW-05).

use super::connect_ask::{ConnectAsk, ConnectCall, ConnectRefusal, ConnectReply};
use calloop::channel::SyncSender;
use std::collections::BTreeMap;
use std::sync::mpsc::TrySendError;
use std::sync::{Mutex, PoisonError};
use std::time::Duration;

/// The calls waiting for the event loop. The loop answers each at its next turn, so a few are
/// plenty; a call beyond them is dropped, never queued.
pub const CONNECT_QUEUE: usize = 8;

/// The calls one caller (by its unique bus name) may make in one [`RATE_WINDOW`]. The shell
/// connects once at start and the portal once per remote session, so four is generous.
pub const CALLS_PER_WINDOW: u32 = 4;

/// The window [`CALLS_PER_WINDOW`] counts in.
pub const RATE_WINDOW: Duration = Duration::from_secs(1);

/// The most callers counted at once. Each costs a short name, so the table stays small; a caller
/// beyond it is refused until the window turns.
pub const TRACKED_CALLERS: usize = 1024;

/// How long a call waits for the event loop's answer before it is given up on.
pub const ANSWER_WAIT: Duration = Duration::from_secs(10);

/// The calls each caller made in the current window. No clock is read: whoever owns the window
/// calls [`RateLimit::new_window`] when it passes.
#[derive(Debug, Default)]
pub struct RateLimit {
    calls: BTreeMap<String, u32>,
}

impl RateLimit {
    /// Counts a call by `name` and says whether it is within the rate (false: over the rate, or
    /// no room to count the caller).
    pub fn admit(&mut self, name: &str) -> bool {
        let full = self.calls.len() >= TRACKED_CALLERS;
        match self.calls.get_mut(name) {
            Some(count) if *count >= CALLS_PER_WINDOW => false,
            Some(count) => {
                *count += 1;
                true
            }
            None if full => false,
            None => {
                self.calls.insert(name.to_owned(), 1);
                true
            }
        }
    }

    /// Forgets the window's counts: every caller may call again.
    pub fn new_window(&mut self) {
        self.calls.clear();
    }

    /// How many callers are counted now.
    pub fn tracked(&self) -> usize {
        self.calls.len()
    }
}

/// The door into the event loop: the rate limit and the bounded queue.
#[derive(Debug)]
pub struct Intake {
    calls: SyncSender<ConnectCall>,
    rate: Mutex<RateLimit>,
}

impl Intake {
    /// The door over `calls`, a channel made with `calloop::channel::sync_channel(CONNECT_QUEUE)`.
    pub fn new(calls: SyncSender<ConnectCall>) -> Intake {
        Intake {
            calls,
            rate: Mutex::new(RateLimit::default()),
        }
    }

    /// Counts a call by the unique bus name `caller`.
    pub fn rate(&self, caller: &str) -> Result<(), ConnectRefusal> {
        let mut rate = self.rate.lock().unwrap_or_else(PoisonError::into_inner);
        if rate.admit(caller) {
            Ok(())
        } else {
            Err(ConnectRefusal::TooMany)
        }
    }

    /// The window passed.
    pub fn new_window(&self) {
        self.rate
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .new_window();
    }

    /// Puts `ask` in the queue and gives back where its answer will arrive. A full queue drops
    /// the call; a loop that is gone is `Unavailable`.
    pub fn enqueue(
        &self,
        ask: ConnectAsk,
    ) -> Result<tokio::sync::oneshot::Receiver<ConnectReply>, ConnectRefusal> {
        let (reply, answer) = tokio::sync::oneshot::channel();
        self.calls
            .try_send(ConnectCall { ask, reply })
            .map_err(|error| match error {
                TrySendError::Full(_) => ConnectRefusal::TooMany,
                TrySendError::Disconnected(_) => ConnectRefusal::Unavailable,
            })?;
        Ok(answer)
    }

    /// How many callers the rate limit counts now.
    pub fn tracked(&self) -> usize {
        self.rate
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .tracked()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dbus::Proven;
    use calloop::channel::sync_channel;
    use casement_core::policy::ConnectionRole;
    use peephole_bus::peephole_core::{AppKey, roles};

    fn ask() -> ConnectAsk {
        let app = AppKey::new("org.quire.Shell").expect("an app");
        ConnectAsk::new(ConnectionRole::Shell, Proven::new(app, roles::SHELL))
    }

    #[test]
    fn a_caller_is_counted_per_window_and_forgiven_when_it_turns() {
        let mut rate = RateLimit::default();
        let first: Vec<bool> = (0..=CALLS_PER_WINDOW).map(|_| rate.admit(":1.1")).collect();
        let want: Vec<bool> = (0..CALLS_PER_WINDOW).map(|_| true).chain([false]).collect();
        assert_eq!(first, want);
        assert!(rate.admit(":1.2"), "another caller");
        rate.new_window();
        assert!(rate.admit(":1.1"));
    }

    /// CASW-05: a flood of calls from a hundred thousand callers holds the queue at its bound and
    /// the counts at theirs, however long it lasts.
    #[test]
    fn a_hundred_thousand_calls_leave_the_queue_and_the_counts_at_their_bounds() {
        let (calls, _channel) = sync_channel::<ConnectCall>(CONNECT_QUEUE);
        let intake = Intake::new(calls);
        let mut waiting = Vec::new();
        let mut dropped = 0_usize;
        for n in 0..100_000_u32 {
            let caller = format!(":1.{n}");
            if intake.rate(&caller).is_err() {
                dropped += 1;
                continue;
            }
            match intake.enqueue(ask()) {
                Ok(answer) => waiting.push(answer),
                Err(why) => {
                    assert_eq!(why, ConnectRefusal::TooMany);
                    dropped += 1;
                }
            }
        }
        assert_eq!(waiting.len(), CONNECT_QUEUE, "the queue holds its bound");
        assert_eq!(
            waiting.len() + dropped,
            100_000,
            "every other call was dropped"
        );
        assert_eq!(
            intake.tracked(),
            TRACKED_CALLERS,
            "the counts stop at their bound"
        );
    }

    #[test]
    fn a_loop_that_is_gone_is_unavailable() {
        let (calls, channel) = sync_channel::<ConnectCall>(CONNECT_QUEUE);
        let intake = Intake::new(calls);
        drop(channel);
        assert_eq!(
            intake.enqueue(ask()).map(|_| ()),
            Err(ConnectRefusal::Unavailable)
        );
    }
}
