//! Serving `org.quire.Compositor1` on the session bus: a thread of its own with a
//! current-thread tokio runtime (casement itself reaches neither zbus nor tokio), and a bounded
//! calloop channel carrying the calls that passed the caller check into casement's event loop.
//! The same shape as `serve_debug_at`.

use super::callers::{CallerCheck, DesktopCallers};
use super::compositor1::{BUS_NAME, Compositor1, OBJECT_PATH};
use super::connect_ask::ConnectCall;
use super::intake::{CONNECT_QUEUE, Intake, RATE_WINDOW};
use super::portal_watch::{self, PortalWatch};
use crate::IpcError;
use crate::privacy::{PrivacyLink, serve_privacy};
use calloop::channel::{Channel, SyncSender, sync_channel};
use peephole_bus::peephole_core::{Proof, Requirement, Table, roles};
use peephole_bus::{BusPeers, ManagerCheck, Tokio};
use std::sync::{Arc, mpsc};
use std::time::Duration;
use zbus::Connection;
use zbus::fdo::RequestNameFlags;

/// Makes the caller check once the bus connection exists.
type BuildCheck = Box<dyn FnOnce(&Connection) -> Arc<dyn CallerCheck> + Send>;

/// How the callers of Compositor1 are told apart: made once the bus connection exists (the
/// identifier asks the bus about its senders), and the window the rate limit counts in.
pub struct CallerSetup {
    build: BuildCheck,
    window: Duration,
    privacy: Option<PrivacyLink>,
}

impl std::fmt::Debug for CallerSetup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CallerSetup")
            .field("window", &self.window)
            .field("privacy", &self.privacy)
            .finish_non_exhaustive()
    }
}

impl CallerSetup {
    /// The real identifier: the system's `/proc` and the unit manager on the bus, judging against
    /// `table` (the shipped caller table). The unit manager's own answers are believed only when
    /// its image is the one the table names for the `manager` role.
    pub fn desktop(table: Table) -> CallerSetup {
        CallerSetup::with(move |connection| {
            let manager = ManagerCheck::Verified(Requirement::new(roles::MANAGER, Proof::Exe));
            let peers: DesktopCallers =
                BusPeers::new(connection.clone(), table, manager).with_offload(Tokio);
            Arc::new(peers)
        })
    }

    /// An identifier `build` makes from the bus connection (a test passes one that is made to
    /// stand as it likes).
    pub fn with(
        build: impl FnOnce(&Connection) -> Arc<dyn CallerCheck> + Send + 'static,
    ) -> CallerSetup {
        CallerSetup {
            build: Box::new(build),
            window: RATE_WINDOW,
            privacy: None,
        }
    }

    /// The same, also serving the Privacy list for Settings (`org.quire.SettingsModule1` at
    /// `PRIVACY_PATH`) over `link`, whose callers are told apart by the same check.
    pub fn with_privacy(self, link: PrivacyLink) -> CallerSetup {
        CallerSetup {
            privacy: Some(link),
            ..self
        }
    }

    /// The same, counting calls per `window` instead of per second (a test makes it long so
    /// the count is exact).
    pub fn window(self, window: Duration) -> CallerSetup {
        CallerSetup { window, ..self }
    }

    /// The same, taking peers that match `table` to run in the units it names (test builds only:
    /// a jailed desktop has no unit manager to say so).
    #[cfg(feature = "debug-inject")]
    pub fn with_test_units(self, table: super::debug_units::PeerUnitTable) -> CallerSetup {
        use super::debug_units::{DebugPeerUnits, ProcComm};
        let build = self.build;
        CallerSetup {
            build: Box::new(move |connection: &Connection| {
                let inner = build(connection);
                let wrapped: Arc<dyn CallerCheck> = Arc::new(DebugPeerUnits::new(
                    table,
                    Arc::new(ProcComm),
                    inner,
                    connection.clone(),
                ));
                wrapped
            }),
            window: self.window,
            privacy: self.privacy,
        }
    }
}

/// Starts serving Compositor1 on the bus at `address` (the session bus; the caller reads it
/// from its environment), telling callers apart as `setup` says. The channel carries the calls
/// for the event loop to answer, at most [`CONNECT_QUEUE`] at a time; the connection is
/// casement's handle on the bus (keep it, or the name is released when the last clone goes).
/// The thread behind it lives as long as the process.
pub fn serve_compositor_at(
    address: &str,
    setup: CallerSetup,
) -> Result<(Channel<ConnectCall>, Connection), IpcError> {
    let (calls, channel) = sync_channel::<ConnectCall>(CONNECT_QUEUE);
    let (ready, started) = mpsc::channel::<Result<Connection, IpcError>>();
    let address = address.to_owned();
    std::thread::Builder::new()
        .name("casement-compositor1".to_owned())
        .spawn(move || run(&address, calls, setup, &ready))
        .map_err(|error| IpcError::Bus(format!("the Compositor1 thread did not start: {error}")))?;
    let connection = started.recv().map_err(|_| {
        IpcError::Bus("the Compositor1 thread ended before it connected".to_owned())
    })??;
    Ok((channel, connection))
}

/// The thread: connect, say so, then drive the runtime for the connection's tasks.
fn run(
    address: &str,
    calls: SyncSender<ConnectCall>,
    setup: CallerSetup,
    ready: &mpsc::Sender<Result<Connection, IpcError>>,
) {
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            let _ = ready.send(Err(IpcError::Bus(format!("tokio runtime: {error}"))));
            return;
        }
    };
    runtime.block_on(async {
        match connect(address, calls, setup).await {
            Ok(connection) => {
                if ready.send(Ok(connection)).is_ok() {
                    std::future::pending::<()>().await;
                }
            }
            Err(error) => {
                let _ = ready.send(Err(IpcError::Bus(error.to_string())));
            }
        }
    });
}

/// The shortest window the rate limit may count in (a zero period would stop the timer).
const SHORTEST_WINDOW: Duration = Duration::from_millis(1);

async fn connect(
    address: &str,
    calls: SyncSender<ConnectCall>,
    setup: CallerSetup,
) -> zbus::Result<Connection> {
    let connection = zbus::connection::Builder::address(address)?.build().await?;
    let window = setup.window.max(SHORTEST_WINDOW);
    let callers = (setup.build)(&connection);
    let privacy = setup.privacy;
    let for_privacy = Arc::clone(&callers);
    let intake = Arc::new(Intake::new(calls));
    let ticks = Arc::clone(&intake);
    tokio::spawn(async move {
        let mut every = tokio::time::interval(window);
        loop {
            every.tick().await;
            ticks.new_window();
        }
    });
    // The object first, then the name: no call can arrive before it is answered. Without
    // replacement and without a queue, two casements on one session is a misconfiguration: the
    // second must fail, not take over.
    let portals = Arc::new(PortalWatch::default());
    tokio::spawn(portal_watch::listen(
        connection.clone(),
        Arc::clone(&portals),
        Arc::clone(&intake),
    ));
    connection
        .object_server()
        .at(OBJECT_PATH, Compositor1::new(intake, callers, portals))
        .await?;
    if let Some(link) = privacy {
        serve_privacy(&connection, for_privacy, link).await?;
    }
    connection
        .request_name_with_flags(BUS_NAME, RequestNameFlags::DoNotQueue.into())
        .await?;
    Ok(connection)
}

#[cfg(test)]
mod tests;
