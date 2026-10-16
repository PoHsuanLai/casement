//! The connection `org.quire.Compositor1.Connect("shell")` hands out, asked of casement on the
//! session bus. The probe's `shell-connect` and `casement-shell-exec` both take theirs here.

use std::os::fd::OwnedFd;
use std::time::Duration;

const BUS_NAME: &str = "org.quire.Compositor1";
const OBJECT_PATH: &str = "/org/quire/Compositor1";
const INTERFACE: &str = "org.quire.Compositor1";
/// How long the bus call may take.
const CALL_PATIENCE: Duration = Duration::from_secs(10);

/// The client end of a new `shell` connection.
pub fn shell() -> Result<OwnedFd, String> {
    let address = std::env::var("DBUS_SESSION_BUS_ADDRESS")
        .map_err(|_| "DBUS_SESSION_BUS_ADDRESS is not set".to_owned())?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;
    let call = async {
        let bus = zbus::connection::Builder::address(address.as_str())?
            .build()
            .await?;
        let proxy = zbus::Proxy::new(&bus, BUS_NAME, OBJECT_PATH, INTERFACE).await?;
        let fd: zbus::zvariant::OwnedFd = proxy.call("Connect", &("shell",)).await?;
        Ok::<OwnedFd, zbus::Error>(fd.into())
    };
    // The timeout's timer is made inside the runtime: tokio's timers need its context.
    runtime
        .block_on(async { tokio::time::timeout(CALL_PATIENCE, call).await })
        .map_err(|_| "Connect did not answer".to_owned())?
        .map_err(|e| e.to_string())
}
