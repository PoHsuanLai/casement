//! `org.quire.Casement.FakeLogind1`: lets a jailed scenario drive the fake from outside the
//! process (`busctl call ... Lock`) and read back what it recorded.

use super::log::Recorder;
use super::manager::SleepPhase;
use super::signals::Signals;
use zbus::fdo;

/// The control object.
#[derive(Debug)]
pub(super) struct Control {
    log: Recorder,
    signals: Signals,
}

impl Control {
    pub(super) fn new(log: Recorder, signals: Signals) -> Control {
        Control { log, signals }
    }
}

fn failed(error: impl std::fmt::Display) -> fdo::Error {
    fdo::Error::Failed(error.to_string())
}

#[zbus::interface(name = "org.quire.Casement.FakeLogind1")]
impl Control {
    /// Emits the session's `Lock` signal.
    async fn lock(&self) -> fdo::Result<()> {
        self.signals.lock().await.map_err(failed)
    }

    /// Emits the session's `Unlock` signal.
    async fn unlock(&self) -> fdo::Result<()> {
        self.signals.unlock().await.map_err(failed)
    }

    /// Emits `PrepareForSleep(starting)`. The wire carries a bool.
    async fn prepare_for_sleep(&self, starting: bool) -> fdo::Result<()> {
        self.signals
            .prepare_for_sleep(SleepPhase::from_starting(starting))
            .await
            .map_err(failed)
    }

    /// The calls recorded so far, one `Debug` line each.
    fn calls(&self) -> Vec<String> {
        self.log
            .calls()
            .iter()
            .map(|call| format!("{call:?}"))
            .collect()
    }

    /// The `what` of each inhibitor still held.
    fn held_inhibitors(&self) -> Vec<String> {
        self.log.held()
    }
}
