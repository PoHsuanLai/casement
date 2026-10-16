//! A request to read the administrator's privacy settings again: `SIGHUP`, which
//! `casementctl privacy reload` sends. Without a handler it would end the process.
//!
//! The same shape as [`crate::stop`]: a thread with a small runtime waits for the signal and
//! sends one message into the event loop, which does the work.

use smithay::reexports::calloop::channel::{self, Channel, Sender};
use std::sync::mpsc;
use std::time::Duration;
use tokio::signal::unix::{SignalKind, signal};

/// How long starting the watcher may take before it is given up on.
const INSTALL_WITHIN: Duration = Duration::from_secs(5);

/// Why the watcher could not start.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum HangupError {
    /// The thread or its runtime could not be made.
    #[error("the reload watcher thread: {0}")]
    Thread(#[source] std::io::Error),
    /// The handler could not be installed.
    #[error("installing the reload handler: {0}")]
    Install(#[source] std::io::Error),
    /// The watcher did not report in time.
    #[error("the reload watcher did not start within {} seconds", INSTALL_WITHIN.as_secs())]
    Timeout,
}

/// A request to read the settings again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReloadRequested;

/// Starts the watcher; the channel yields one [`ReloadRequested`] for each `SIGHUP`. Returns
/// once the handler is installed, so a signal sent after this is heard.
pub fn watch() -> Result<Channel<ReloadRequested>, HangupError> {
    let (sender, channel) = channel::channel();
    let (ready_tx, ready) = mpsc::channel();
    std::thread::Builder::new()
        .name("casement-reload".to_owned())
        .spawn(move || run(&sender, &ready_tx))
        .map_err(HangupError::Thread)?;
    match ready.recv_timeout(INSTALL_WITHIN) {
        Ok(Ok(())) => Ok(channel),
        Ok(Err(error)) => Err(error),
        Err(_) => Err(HangupError::Timeout),
    }
}

fn run(sender: &Sender<ReloadRequested>, ready: &mpsc::Sender<Result<(), HangupError>>) {
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            let _ = ready.send(Err(HangupError::Thread(error)));
            return;
        }
    };
    runtime.block_on(async {
        let mut hangup = match signal(SignalKind::hangup()) {
            Ok(hangup) => hangup,
            Err(error) => {
                let _ = ready.send(Err(HangupError::Install(error)));
                return;
            }
        };
        let _ = ready.send(Ok(()));
        // A stream that ended cannot be heard again; a loop that is gone needs no word.
        while hangup.recv().await.is_some() {
            if sender.send(ReloadRequested).is_err() {
                return;
            }
        }
    });
}
