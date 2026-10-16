//! Whether a window's accessibility root belongs to the window's own program (PORTAL-15).
//!
//! `quire_window.set_a11y_root(bus_name, object_path)` names an object any program on the bus
//! could own. The assistant tier reads that object to see the window, so a client must not be
//! able to point it at someone else's. [`verify`] asks the bus which process owns the name and
//! accepts the root only when that process is the client's own: the pid casement holds the client
//! by. A sandboxed client's bus connection is its proxy's, a different process, so it never
//! verifies; its tree is not offered to the assistant tier, which is the safe default.
//!
//! [`A11yWorker`] does the asking on a thread of its own (the event loop never waits on the bus)
//! and answers down a channel with the root the model may adopt: [`A11yRoot::At`] when verified,
//! [`A11yRoot::Unknown`] otherwise. Until the answer arrives the window's root is `Unknown`.

use calloop::channel::{Channel, SyncSender, sync_channel};
use casement_core::WindowId;
use casement_core::window::A11yRoot;
use std::time::Duration;
use tokio::sync::mpsc::{Receiver, Sender, channel, error::TrySendError};
use zbus::Connection;
use zbus::fdo::DBusProxy;
use zbus::names::BusName;

/// The most roots waiting to be checked, and the most answers waiting for the event loop. A root
/// asked beyond it stays `Unknown` and is asked again when the client sets it again.
pub const A11Y_QUEUE: usize = 32;

/// How long the bus may take to say who owns a name before the root stays `Unknown`.
pub const OWNER_WAIT: Duration = Duration::from_secs(2);

/// One root to check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct A11yAsk {
    /// The window whose client set the root.
    pub window: WindowId,
    /// The client's process id, as casement holds the client (its pinned peer).
    pub client_pid: u32,
    /// The bus name the client gave, unique or well known.
    pub bus_name: String,
    /// The object path the client gave.
    pub object_path: String,
}

/// What the check came to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct A11yVerdict {
    /// The window.
    pub window: WindowId,
    /// The root the model may adopt.
    pub root: A11yRoot,
}

/// Why a root could not be queued.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum A11yFault {
    /// Too many roots are waiting.
    Busy,
    /// The thread has stopped.
    Stopped,
}

/// The root `ask` may have: its own, when the process that owns the name is the client's.
pub async fn verify(connection: &Connection, ask: &A11yAsk) -> A11yRoot {
    let Ok(name) = BusName::try_from(ask.bus_name.as_str()) else {
        return A11yRoot::Unknown;
    };
    let Ok(bus) = DBusProxy::new(connection).await else {
        return A11yRoot::Unknown;
    };
    let owner = tokio::time::timeout(OWNER_WAIT, bus.get_connection_unix_process_id(name)).await;
    match owner {
        Ok(Ok(pid)) if pid == ask.client_pid => A11yRoot::At {
            bus_name: ask.bus_name.clone(),
            object_path: ask.object_path.clone(),
        },
        _ => A11yRoot::Unknown,
    }
}

/// The thread that checks roots: a current-thread runtime of its own, the bus connected on the
/// first root.
#[derive(Debug)]
pub struct A11yWorker {
    queue: Sender<A11yAsk>,
}

impl A11yWorker {
    /// Starts the thread, checking against the bus at `address` (the session bus). The channel
    /// carries the answers to the event loop. The thread ends when the worker is dropped.
    pub fn start(address: String) -> Result<(A11yWorker, Channel<A11yVerdict>), std::io::Error> {
        let (queue, incoming) = channel(A11Y_QUEUE);
        let (verdicts, answers) = sync_channel(A11Y_QUEUE);
        std::thread::Builder::new()
            .name("casement-a11y".to_owned())
            .spawn(move || run(&address, incoming, &verdicts))?;
        Ok((A11yWorker { queue }, answers))
    }

    /// Queues `ask`; refused when too many are waiting or the thread has stopped.
    pub fn check(&self, ask: A11yAsk) -> Result<(), A11yFault> {
        self.queue.try_send(ask).map_err(|error| match error {
            TrySendError::Full(_) => A11yFault::Busy,
            TrySendError::Closed(_) => A11yFault::Stopped,
        })
    }
}

fn run(address: &str, mut incoming: Receiver<A11yAsk>, verdicts: &SyncSender<A11yVerdict>) {
    let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    else {
        // No runtime: dropping the receiver makes every `check` fail as `Stopped`.
        return;
    };
    runtime.block_on(async move {
        let mut connection: Option<Connection> = None;
        while let Some(ask) = incoming.recv().await {
            let root = match connected(&mut connection, address).await {
                Ok(bus) => verify(&bus, &ask).await,
                Err(_) => A11yRoot::Unknown,
            };
            // Blocks this thread only, when the event loop is behind; a loop that is gone ends it.
            if verdicts
                .send(A11yVerdict {
                    window: ask.window,
                    root,
                })
                .is_err()
            {
                return;
            }
        }
    });
}

/// The bus, connected the first time it is asked for.
async fn connected(slot: &mut Option<Connection>, address: &str) -> zbus::Result<Connection> {
    if let Some(bus) = slot {
        return Ok(bus.clone());
    }
    let bus = zbus::connection::Builder::address(address)?.build().await?;
    *slot = Some(bus.clone());
    Ok(bus)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_bus::{PrivateBus, name_of};

    /// What a check is expected to come to.
    #[derive(Clone, Copy)]
    enum Comes {
        Verified,
        Unknown,
    }

    fn ask(pid: u32, bus_name: &str) -> A11yAsk {
        A11yAsk {
            window: WindowId(4),
            client_pid: pid,
            bus_name: bus_name.to_owned(),
            object_path: "/org/a11y/root".to_owned(),
        }
    }

    /// A root is the client's own only when the process that owns the name is the client's. On a
    /// private bus every connection of this test is this process, so a client that is some other
    /// process is the pid that is not ours.
    #[tokio::test]
    async fn a_root_verifies_only_when_the_owner_is_the_clients_own_process() {
        let bus = PrivateBus::start();
        let owner = bus.client().await;
        let (worker, answers) = A11yWorker::start(bus.address.clone()).expect("a thread");
        let ours = std::process::id();
        let theirs = ours.wrapping_add(1);
        let unique = name_of(&owner);
        let cases = [
            ("its own unique name", ask(ours, &unique), Comes::Verified),
            (
                "someone else's process",
                ask(theirs, &unique),
                Comes::Unknown,
            ),
            ("a name nobody owns", ask(ours, ":1.987654"), Comes::Unknown),
            (
                "a well-known name nobody owns",
                ask(ours, "org.example.Nobody"),
                Comes::Unknown,
            ),
            ("not a bus name", ask(ours, "no good"), Comes::Unknown),
        ];
        for (case, asked, comes) in cases {
            worker.check(asked.clone()).expect("queued");
            let verdict = answers.recv().expect("an answer");
            assert_eq!(verdict.window, WindowId(4), "{case}");
            let want = match comes {
                Comes::Verified => A11yRoot::At {
                    bus_name: asked.bus_name,
                    object_path: asked.object_path,
                },
                Comes::Unknown => A11yRoot::Unknown,
            };
            assert_eq!(verdict.root, want, "{case}");
        }
    }
}
