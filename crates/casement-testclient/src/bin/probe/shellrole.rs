//! The shell side of the session: the connection `org.quire.Compositor1.Connect("shell")` hands
//! out, and the `quire_shell_v1` requests made on it (roles and minimise targets). The grammar
//! is in `command.rs`; every verb answers one JSON note.

use casement_testclient::quire::shell::{ShellHandle, ShellRole};
use casement_testclient::{ClientError, TestClient};
use serde_json::{Value, json};
use std::os::fd::OwnedFd;
use std::time::Duration;
use wayland_client::protocol::wl_surface::WlSurface;

const BUS_NAME: &str = "org.quire.Compositor1";
const OBJECT_PATH: &str = "/org/quire/Compositor1";
const INTERFACE: &str = "org.quire.Compositor1";
/// How long the bus call may take.
const CALL_PATIENCE: Duration = Duration::from_secs(10);
/// Poll rounds (100 ms each) the first roundtrip on the new connection may take.
const ROUNDS: u32 = 100;

/// What the session keeps once it is the shell.
pub(crate) struct Shell {
    handle: ShellHandle,
    /// The surface that was given the dock role: minimise targets are local to it.
    dock: Option<WlSurface>,
}

/// The client end of a new `shell` connection, asked of casement on the session bus.
fn hand_out() -> Result<OwnedFd, String> {
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
    runtime
        .block_on(tokio::time::timeout(CALL_PATIENCE, call))
        .map_err(|_| "Connect did not answer".to_owned())?
        .map_err(|e| e.to_string())
}

/// A client over `fd` that has had its first roundtrip (the registry is filled).
fn client_over(fd: OwnedFd) -> Result<TestClient, ClientError> {
    let mut client = TestClient::over(fd)?;
    let sync = client.sync()?;
    for _ in 0..ROUNDS {
        client.pump();
        if sync.is_done() {
            return Ok(client);
        }
        if !client.is_connected() {
            break;
        }
        let wire = client.wait_fd();
        let mut fds = [rustix::event::PollFd::new(
            &wire,
            rustix::event::PollFlags::IN,
        )];
        let slice = rustix::event::Timespec {
            tv_sec: 0,
            tv_nsec: 100_000_000,
        };
        let _ = rustix::event::poll(&mut fds, Some(&slice));
    }
    Err(ClientError::Dispatch(
        "the shell connection did not answer the first roundtrip".to_owned(),
    ))
}

/// Connects as the shell: the new client, and the bound `quire_shell_v1` on it.
pub(crate) fn connect() -> Result<(TestClient, Shell), String> {
    let mut client = client_over(hand_out()?).map_err(|e| e.to_string())?;
    let advertised = client
        .globals()
        .contents()
        .clone_list()
        .iter()
        .any(|global| global.interface == "quire_shell_v1");
    if !advertised {
        return Err("the connection does not advertise quire_shell_v1".to_owned());
    }
    let handle = ShellHandle::bind(&mut client, 1);
    Ok((client, Shell { handle, dock: None }))
}

impl Shell {
    /// Gives `surface` the role `role`.
    pub(crate) fn role(&mut self, role: ShellRole, surface: &WlSurface) -> Value {
        self.handle.set_role(surface, role);
        if role == ShellRole::Dock {
            self.dock = Some(surface.clone());
        }
        json!({ "shell": "role", "role": format!("{role:?}").to_lowercase() })
    }

    /// Sets where `toplevel` minimises to: a rectangle local to the dock.
    pub(crate) fn target(
        &self,
        toplevel: &str,
        at: (i32, i32),
        size: (u32, u32),
    ) -> Result<Value, String> {
        let dock = self
            .dock
            .as_ref()
            .ok_or("no dock yet: run shell-role dock LAYER first")?;
        self.handle.set_minimise_target(dock, toplevel, at, size);
        Ok(json!({ "shell": "minimise-target", "toplevel": toplevel,
            "rect": [at.0, at.1, size.0, size.1] }))
    }

    /// Forgets the minimise target of `toplevel`.
    pub(crate) fn clear(&self, toplevel: &str) -> Value {
        self.handle.clear_minimise_target(toplevel);
        json!({ "shell": "minimise-clear", "toplevel": toplevel })
    }
}
