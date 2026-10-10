//! The thread that makes the calls: a current-thread tokio runtime of its own (casement itself
//! reaches neither zbus nor tokio), the session bus connected on the first call, and one task
//! per call so a slow reply holds nobody up.

use super::{FailedCall, ShellCallError};
use casement_core::input::DbusCall;
use std::sync::mpsc;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use zbus::Connection;

/// The most string arguments a binding's call may carry.
pub(super) const MOST_ARGS: usize = 4;

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
            let report = report.clone();
            tokio::spawn(async move {
                if let Err(why) = deliver(&bus, &call).await {
                    let _ = report.send(FailedCall {
                        call,
                        why: ShellCallError::Bus(why),
                    });
                }
            });
        }
    });
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
