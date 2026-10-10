//! Serving `org.quire.Compositor1` on the session bus: a thread of its own with a
//! current-thread tokio runtime (casement itself reaches neither zbus nor tokio), and a calloop
//! channel carrying every call into casement's event loop. The same shape as `serve_debug_at`.

use super::compositor1::{BUS_NAME, Compositor1, ConnectCall, OBJECT_PATH, PeerCgroups};
use crate::IpcError;
use calloop::channel::{Channel, channel};
use std::sync::{Arc, mpsc};
use zbus::Connection;

/// Starts serving Compositor1 on the bus at `address` (the session bus; the caller reads it
/// from its environment). The channel carries the calls for the event loop to answer; the
/// connection is casement's handle on the bus (keep it, or the name is released when the last
/// clone goes). The thread behind it lives as long as the process.
pub fn serve_compositor_at(
    address: &str,
    cgroups: Arc<dyn PeerCgroups>,
) -> Result<(Channel<ConnectCall>, Connection), IpcError> {
    let (calls, channel) = channel::<ConnectCall>();
    let (ready, started) = mpsc::channel::<Result<Connection, IpcError>>();
    let address = address.to_owned();
    std::thread::Builder::new()
        .name("casement-compositor1".to_owned())
        .spawn(move || run(&address, calls, cgroups, &ready))
        .map_err(|error| IpcError::Bus(format!("the Compositor1 thread did not start: {error}")))?;
    let connection = started.recv().map_err(|_| {
        IpcError::Bus("the Compositor1 thread ended before it connected".to_owned())
    })??;
    Ok((channel, connection))
}

/// The thread: connect, say so, then drive the runtime for the connection's tasks.
fn run(
    address: &str,
    calls: calloop::channel::Sender<ConnectCall>,
    cgroups: Arc<dyn PeerCgroups>,
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
        match connect(address, calls, cgroups).await {
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

async fn connect(
    address: &str,
    calls: calloop::channel::Sender<ConnectCall>,
    cgroups: Arc<dyn PeerCgroups>,
) -> zbus::Result<Connection> {
    zbus::connection::Builder::address(address)?
        // Two casements on one session is a misconfiguration: the second must fail, not take over.
        .allow_name_replacements(false)
        .replace_existing_names(false)
        .name(BUS_NAME)?
        .serve_at(OBJECT_PATH, Compositor1::new(calls, cgroups))?
        .build()
        .await
}
