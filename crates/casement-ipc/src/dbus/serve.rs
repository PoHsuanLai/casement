//! Serving `Debug1` on the private bus: a thread of its own with a current-thread tokio runtime
//! (casement itself reaches neither zbus nor tokio), and a calloop channel carrying every call
//! into casement's event loop.

use super::debug1::{Debug1, DebugCall, OBJECT_PATH};
use crate::IpcError;
use calloop::channel::{Channel, channel};
use std::sync::mpsc;
use zbus::Connection;

/// Starts serving Debug1 on the bus at `address` (the jail's private one; the caller reads it
/// from its environment). The channel carries the calls for the event loop to answer; the
/// connection is casement's handle on the bus (keep it, or the name is released when the last
/// clone goes). The thread behind it lives as long as the process.
pub fn serve_debug_at(address: &str) -> Result<(Channel<DebugCall>, Connection), IpcError> {
    let (calls, channel) = channel::<DebugCall>();
    let (ready, started) = mpsc::channel::<Result<Connection, IpcError>>();
    let address = address.to_owned();
    std::thread::Builder::new()
        .name("casement-debug1".to_owned())
        .spawn(move || run(&address, calls, &ready))
        .map_err(|error| IpcError::Bus(format!("the Debug1 thread did not start: {error}")))?;
    let connection = started
        .recv()
        .map_err(|_| IpcError::Bus("the Debug1 thread ended before it connected".to_owned()))??;
    Ok((channel, connection))
}

/// The thread: connect, say so, then drive the runtime for the connection's tasks.
fn run(
    address: &str,
    calls: calloop::channel::Sender<DebugCall>,
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
        match connect(address, calls).await {
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
    calls: calloop::channel::Sender<DebugCall>,
) -> zbus::Result<Connection> {
    zbus::connection::Builder::address(address)?
        // Two casements on one bus is a misconfiguration: the second must fail, not take over.
        .allow_name_replacements(false)
        .replace_existing_names(false)
        .name(Debug1::BUS_NAME)?
        .serve_at(OBJECT_PATH, Debug1::new(calls))?
        .build()
        .await
}

#[cfg(test)]
mod tests;
