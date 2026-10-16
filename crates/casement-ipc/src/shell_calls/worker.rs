//! The thread that makes the calls: a current-thread tokio runtime of its own (casement itself
//! reaches neither zbus nor tokio), the session bus connected on the first call, and one task
//! per call so a slow reply holds nobody up. A call waits for its reply at most
//! [`CALL_TIMEOUT`], and at most [`MOST_IN_FLIGHT`] calls wait at once (CASW-16): a target that
//! hangs gathers neither parked tasks nor silence, it gets failures reported.

use super::{FailedCall, ShellCallError};
use casement_core::input::DbusCall;
use std::future::Future;
use std::sync::{Arc, mpsc};
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use zbus::Connection;

/// The most string arguments a binding's call may carry.
pub(super) const MOST_ARGS: usize = 4;

/// How long a call waits for its reply: a shell that has not answered a key binding's call in
/// this long is hung, and the call is reported failed instead of parking a task for the bus
/// daemon's own, much longer, limit.
pub(super) const CALL_TIMEOUT: Duration = Duration::from_secs(5);

/// The most calls waiting for a reply at once: a bound key held down repeats, and against a
/// hung target each repeat would otherwise park one more task.
pub(super) const MOST_IN_FLIGHT: usize = 16;

/// A running worker: calls go in, failures come back.
#[derive(Debug)]
pub(super) struct Worker {
    calls: UnboundedSender<DbusCall>,
    failed: mpsc::Receiver<FailedCall>,
}

impl Worker {
    /// Starts the thread. It ends when the worker is dropped.
    pub(super) fn start() -> Result<Worker, std::io::Error> {
        let (calls, incoming) = unbounded_channel();
        let (report, failed) = mpsc::channel();
        std::thread::Builder::new()
            .name("casement-shell-calls".to_owned())
            .spawn(move || run(incoming, report))?;
        Ok(Worker { calls, failed })
    }

    /// Hands `call` to the thread; the call comes back when the thread is gone.
    pub(super) fn send(&self, call: DbusCall) -> Result<(), DbusCall> {
        self.calls.send(call).map_err(|error| error.0)
    }

    /// The failures reported since the last time.
    pub(super) fn failures(&self) -> Vec<FailedCall> {
        self.failed.try_iter().collect()
    }
}

fn run(mut incoming: UnboundedReceiver<DbusCall>, report: mpsc::Sender<FailedCall>) {
    let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    else {
        // No runtime, nothing to call with: dropping the receiver makes every send fail, which
        // the caller reports and answers with a new thread.
        return;
    };
    runtime.block_on(async move {
        let mut connection: Option<Connection> = None;
        let permits = Arc::new(Semaphore::new(MOST_IN_FLIGHT));
        while let Some(call) = incoming.recv().await {
            let bus = match connected(&mut connection).await {
                Ok(bus) => bus,
                Err(why) => {
                    let _ = report.send(FailedCall {
                        call,
                        why: ShellCallError::Bus(why),
                    });
                    continue;
                }
            };
            let Ok(permit) = permits.clone().try_acquire_owned() else {
                let _ = report.send(FailedCall {
                    call,
                    why: ShellCallError::Overloaded {
                        most: MOST_IN_FLIGHT,
                    },
                });
                continue;
            };
            let report = report.clone();
            tokio::spawn(async move {
                // The place in the line is held until the reply, the failure or the timeout.
                let _permit = permit;
                if let Err(why) = bounded(CALL_TIMEOUT, deliver(&bus, &call)).await {
                    let _ = report.send(FailedCall { call, why });
                }
            });
        }
    });
}

/// `reply` awaited for at most `limit`: a failure, or [`ShellCallError::TimedOut`].
async fn bounded(
    limit: Duration,
    reply: impl Future<Output = zbus::Result<()>>,
) -> Result<(), ShellCallError> {
    match tokio::time::timeout(limit, reply).await {
        Ok(reply) => reply.map_err(ShellCallError::Bus),
        Err(_) => Err(ShellCallError::TimedOut { after: limit }),
    }
}

/// The session bus, connected the first time it is asked for.
async fn connected(slot: &mut Option<Connection>) -> zbus::Result<Connection> {
    if let Some(bus) = slot {
        return Ok(bus.clone());
    }
    let bus = Connection::session().await?;
    *slot = Some(bus.clone());
    Ok(bus)
}

/// One call, string arguments in order, and its reply awaited (an error reply is a failure).
async fn deliver(bus: &Connection, call: &DbusCall) -> zbus::Result<()> {
    let (service, path, interface, method) = (
        call.service.as_str(),
        call.path.as_str(),
        call.interface.as_str(),
        call.method.as_str(),
    );
    let args: Vec<&str> = call.args.iter().map(String::as_str).collect();
    let reply = match args.as_slice() {
        [] => {
            bus.call_method(Some(service), path, Some(interface), method, &())
                .await
        }
        [a] => {
            bus.call_method(Some(service), path, Some(interface), method, &(*a,))
                .await
        }
        [a, b] => {
            bus.call_method(Some(service), path, Some(interface), method, &(*a, *b))
                .await
        }
        [a, b, c] => {
            bus.call_method(Some(service), path, Some(interface), method, &(*a, *b, *c))
                .await
        }
        [a, b, c, d, ..] => {
            bus.call_method(
                Some(service),
                path,
                Some(interface),
                method,
                &(*a, *b, *c, *d),
            )
            .await
        }
    };
    reply.map(drop)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A reply that never comes is a timeout, not a task parked for good; one that comes is
    /// passed through.
    #[tokio::test]
    async fn a_reply_that_never_comes_is_a_timeout() {
        let hung = bounded(Duration::ZERO, std::future::pending()).await;
        assert!(
            matches!(hung, Err(ShellCallError::TimedOut { .. })),
            "{hung:?}"
        );
        let answered = bounded(CALL_TIMEOUT, std::future::ready(Ok(()))).await;
        assert!(answered.is_ok(), "{answered:?}");
    }

    /// Past [`MOST_IN_FLIGHT`] calls waiting, the next is refused; a call that ends frees its
    /// place.
    #[test]
    fn calls_waiting_for_a_reply_are_limited() {
        let permits = Arc::new(Semaphore::new(MOST_IN_FLIGHT));
        let mut waiting: Vec<_> = (0..MOST_IN_FLIGHT)
            .map(|_| permits.clone().try_acquire_owned().expect("a place"))
            .collect();
        assert!(
            permits.clone().try_acquire_owned().is_err(),
            "no place left"
        );
        waiting.pop();
        assert!(
            permits.clone().try_acquire_owned().is_ok(),
            "a place is free again"
        );
    }
}
