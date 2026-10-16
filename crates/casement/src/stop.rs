//! A request to stop: `SIGTERM` (the service manager's stop, a shutdown) and `SIGINT`.
//!
//! Without a handler these end the process on the spot, so the service manager is never told the
//! compositor is stopping and nothing is closed in order. A thread with a small runtime waits for
//! them and sends one message into the event loop, which then ends its run the ordinary way
//! (clients are closed, `STOPPING=1` is sent, the Spaces file is written).

use smithay::reexports::calloop::channel::{self, Channel, Sender};
use std::sync::mpsc;
use std::time::Duration;
use tokio::signal::unix::{SignalKind, signal};

/// How long starting the watcher may take before it is given up on.
const INSTALL_WITHIN: Duration = Duration::from_secs(5);

/// Why the watcher could not start.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum StopError {
    /// The thread or its runtime could not be made.
    #[error("the stop watcher thread: {0}")]
    Thread(#[source] std::io::Error),
    /// A handler could not be installed.
    #[error("installing the stop handlers: {0}")]
    Install(#[source] std::io::Error),
    /// The watcher did not report in time.
    #[error("the stop watcher did not start within {} seconds", INSTALL_WITHIN.as_secs())]
    Timeout,
}

/// A request to stop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StopRequested;

/// Starts the watcher; the channel yields one [`StopRequested`] when `SIGTERM` or `SIGINT`
/// arrives (again for each later one). Returns once the handlers are installed, so a signal sent
/// after this is heard. The thread lives for the process.
pub fn watch() -> Result<Channel<StopRequested>, StopError> {
    let (sender, channel) = channel::channel();
    let (ready_tx, ready) = mpsc::channel();
    std::thread::Builder::new()
        .name("casement-stop".to_owned())
        .spawn(move || run(&sender, &ready_tx))
        .map_err(StopError::Thread)?;
    match ready.recv_timeout(INSTALL_WITHIN) {
        Ok(Ok(())) => Ok(channel),
        Ok(Err(error)) => Err(error),
        Err(_) => Err(StopError::Timeout),
    }
}

fn run(sender: &Sender<StopRequested>, ready: &mpsc::Sender<Result<(), StopError>>) {
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            let _ = ready.send(Err(StopError::Thread(error)));
            return;
        }
    };
    runtime.block_on(async {
        let streams = signal(SignalKind::terminate())
            .and_then(|term| signal(SignalKind::interrupt()).map(|interrupt| (term, interrupt)));
        let (mut term, mut interrupt) = match streams {
            Ok(streams) => streams,
            Err(error) => {
                let _ = ready.send(Err(StopError::Install(error)));
                return;
            }
        };
        let _ = ready.send(Ok(()));
        loop {
            let heard = tokio::select! {
                heard = term.recv() => heard,
                heard = interrupt.recv() => heard,
            };
            // A stream that ended cannot be heard again; a loop that is gone needs no word.
            if heard.is_none() || sender.send(StopRequested).is_err() {
                return;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sigterm_reaches_the_loop_as_a_message_instead_of_ending_the_process() {
        let channel = watch().expect("the handlers install");
        rustix::process::kill_process(rustix::process::getpid(), rustix::process::Signal::TERM)
            .expect("signal ourselves");
        // The message arrives through the channel's own source; read it by polling the sender
        // side with a loop turn.
        let mut event_loop: smithay::reexports::calloop::EventLoop<'static, Option<StopRequested>> =
            smithay::reexports::calloop::EventLoop::try_new().expect("a loop");
        event_loop
            .handle()
            .insert_source(channel, |event, (), heard| {
                if let channel::Event::Msg(message) = event {
                    *heard = Some(message);
                }
            })
            .expect("inserted");
        let mut heard = None;
        for _ in 0..200 {
            event_loop
                .dispatch(Some(Duration::from_millis(25)), &mut heard)
                .expect("a turn");
            if heard.is_some() {
                break;
            }
        }
        assert_eq!(heard, Some(StopRequested));
    }
}
