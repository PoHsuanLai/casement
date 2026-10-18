//! The stop-and-join every watcher thread shares.

use std::thread::JoinHandle;
use tokio::sync::oneshot;

/// A running watcher thread; dropping it tells the thread to stop and waits for it.
#[derive(Debug)]
pub(crate) struct StopOnDrop {
    stop: Option<oneshot::Sender<()>>,
    thread: Option<JoinHandle<()>>,
}

impl StopOnDrop {
    /// The guard of `thread`, which stops when `stop` fires.
    pub(crate) fn new(stop: oneshot::Sender<()>, thread: JoinHandle<()>) -> StopOnDrop {
        StopOnDrop {
            stop: Some(stop),
            thread: Some(thread),
        }
    }
}

impl Drop for StopOnDrop {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
