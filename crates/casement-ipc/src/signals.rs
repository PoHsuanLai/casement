//! Process signals as the test hooks of `--drm-node` mode (P1-V2): `SIGUSR1` is "the session was
//! paused", `SIGUSR2` is "the session was activated" (a VT switch away and back, simulated with
//! no VT touched) and `SIGHUP` is "read one frame back". A thread with a current-thread tokio
//! runtime waits for them and calls the [`SignalHandler`]; the compositor's loop never installs
//! a signal handler itself.
//!
//! ```no_run
//! use casement_ipc::signals::{SignalHandler, SignalWatch};
//!
//! struct Say;
//! impl SignalHandler for Say {
//!     fn paused(&mut self) { println!("paused"); }
//!     fn activated(&mut self) { println!("activated"); }
//!     fn capture(&mut self) { println!("capture"); }
//! }
//! let _watch = SignalWatch::start(Say).expect("signals can be installed");
//! ```

use std::sync::mpsc;
use std::thread::JoinHandle;
use std::time::Duration;
use tokio::signal::unix::{Signal, SignalKind, signal};
use tokio::sync::oneshot;

/// How long [`SignalWatch::start`] waits for the handlers to be installed.
const INSTALL_WITHIN: Duration = Duration::from_secs(5);

/// What the watcher tells the compositor.
pub trait SignalHandler: Send + 'static {
    /// `SIGUSR1`: the session was paused.
    fn paused(&mut self);

    /// `SIGUSR2`: the session was activated.
    fn activated(&mut self);

    /// `SIGHUP`: read one frame back.
    fn capture(&mut self);
}

/// Why the watcher could not start.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SignalError {
    /// The thread or its runtime could not be started.
    #[error("the signal watcher thread: {0}")]
    Thread(#[source] std::io::Error),
    /// A signal handler could not be installed.
    #[error("installing the signal handlers: {0}")]
    Install(#[source] std::io::Error),
    /// The thread did not report in time.
    #[error("the signal watcher did not start within {} seconds", INSTALL_WITHIN.as_secs())]
    Timeout,
}

/// The running watcher; dropping it stops the thread. The handlers stay installed for the life
/// of the process (the signals are then ignored, not fatal).
#[derive(Debug)]
pub struct SignalWatch {
    stop: Option<oneshot::Sender<()>>,
    thread: Option<JoinHandle<()>>,
}

impl SignalWatch {
    /// Installs the handlers and starts the thread. Returns once they are installed, so a
    /// signal sent after this is heard.
    pub fn start(handler: impl SignalHandler) -> Result<SignalWatch, SignalError> {
        let (stop, stopped) = oneshot::channel();
        let (ready_tx, ready) = mpsc::channel();
        let thread = std::thread::Builder::new()
            .name("casement-signals".to_owned())
            .spawn(move || run(handler, stopped, ready_tx))
            .map_err(SignalError::Thread)?;
        match ready.recv_timeout(INSTALL_WITHIN) {
            Ok(Ok(())) => Ok(SignalWatch {
                stop: Some(stop),
                thread: Some(thread),
            }),
            Ok(Err(error)) => Err(error),
            Err(_) => Err(SignalError::Timeout),
        }
    }
}

impl Drop for SignalWatch {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// The three signal streams.
fn install() -> std::io::Result<(Signal, Signal, Signal)> {
    Ok((
        signal(SignalKind::user_defined1())?,
        signal(SignalKind::user_defined2())?,
        signal(SignalKind::hangup())?,
    ))
}

/// The thread: a current-thread runtime, the three signal streams, the loop.
fn run(
    mut handler: impl SignalHandler,
    mut stopped: oneshot::Receiver<()>,
    ready: mpsc::Sender<Result<(), SignalError>>,
) {
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            let _ = ready.send(Err(SignalError::Thread(error)));
            return;
        }
    };
    runtime.block_on(async move {
        let (mut usr1, mut usr2, mut hup) = match install() {
            Ok(streams) => streams,
            Err(error) => {
                let _ = ready.send(Err(SignalError::Install(error)));
                return;
            }
        };
        let _ = ready.send(Ok(()));
        loop {
            tokio::select! {
                _ = &mut stopped => return,
                heard = usr1.recv() => match heard {
                    // A stream that ended cannot be heard again.
                    Some(()) => handler.paused(),
                    None => return,
                },
                heard = usr2.recv() => match heard {
                    Some(()) => handler.activated(),
                    None => return,
                },
                heard = hup.recv() => match heard {
                    Some(()) => handler.capture(),
                    None => return,
                },
            }
        }
    });
}
