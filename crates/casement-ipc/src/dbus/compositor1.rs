//! `org.quire.Compositor1` at `/org/quire/Compositor1` (the session bus; casement owns the
//! name): the way the shell and the agent tier get their privileged Wayland connections.
//!
//! ```text
//! Connect(role: s) -> (h)    role is "shell" or "agent"; the reply is the client end of a
//!                            new connection that has that role (I7)
//! ```
//!
//! The caller's systemd unit decides: this object reads the caller's cgroup (through the
//! [`PeerCgroups`] seam), hands the question to casement's event loop down a channel, and
//! turns the loop's answer into the reply or a D-Bus error. The loop asks
//! `casement_agent::hand_out`, which checks the unit against the role (`sill.service` the
//! shell, `cuad.service` the agent tier) and makes the socket pair, and inserts the server end
//! into the display. P4-Z adds `Desktop() -> s`, `Perform(action: s) -> s` and the `Killed()`
//! signal. Filled by P2-Q2.

use crate::IpcError;
use casement_core::policy::ConnectionRole;
use std::os::fd::OwnedFd;
use std::sync::Arc;
use zbus::message::Header;
use zbus::names::BusName;
use zbus::{Connection, fdo};

/// The object's path.
pub const OBJECT_PATH: &str = "/org/quire/Compositor1";

/// The well-known name casement owns on the session bus.
pub const BUS_NAME: &str = "org.quire.Compositor1";

/// The object: a channel into the event loop, where every call is carried out, and the seam
/// that tells which control group a bus peer's process is in.
#[derive(Debug)]
pub struct Compositor1 {
    calls: calloop::channel::Sender<ConnectCall>,
    cgroups: Arc<dyn PeerCgroups>,
}

impl Compositor1 {
    /// The object over `calls`, reading peers' control groups through `cgroups`.
    pub fn new(
        calls: calloop::channel::Sender<ConnectCall>,
        cgroups: Arc<dyn PeerCgroups>,
    ) -> Compositor1 {
        Compositor1 { calls, cgroups }
    }
}

/// Which control group a process is in: the text of its `/proc/<pid>/cgroup`. A seam, because
/// reading `/proc` is the only part of a hand-out a test must not do for real (the test kit
/// exports `FakeCgroups`).
pub trait PeerCgroups: std::fmt::Debug + Send + Sync + 'static {
    /// The cgroup file's text for process `pid`.
    fn cgroup_of(&self, pid: u32) -> Result<String, IpcError>;
}

/// The real seam: `/proc/<pid>/cgroup`.
#[derive(Debug, Clone, Copy, Default)]
pub struct ProcCgroups;

impl PeerCgroups for ProcCgroups {
    fn cgroup_of(&self, pid: u32) -> Result<String, IpcError> {
        std::fs::read_to_string(format!("/proc/{pid}/cgroup"))
            .map_err(|error| IpcError::Bus(format!("the control group of process {pid}: {error}")))
    }
}

/// One `Connect` call, with where its answer goes.
#[derive(Debug)]
pub struct ConnectCall {
    /// What was asked.
    pub ask: ConnectAsk,
    /// Where the event loop sends the answer.
    pub reply: tokio::sync::oneshot::Sender<ConnectReply>,
}

/// What a caller asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ConnectAsk {
    /// The role it wants a connection to have.
    pub role: ConnectionRole,
    /// The text of the caller's `/proc/<pid>/cgroup`, from which the loop reads its unit.
    pub caller_cgroup: String,
}

impl ConnectAsk {
    /// The question of a caller whose control group file reads `caller_cgroup`, asking for a
    /// connection with `role`.
    ///
    /// ```
    /// use casement_core::policy::ConnectionRole;
    /// use casement_ipc::dbus::ConnectAsk;
    ///
    /// let ask = ConnectAsk::new(ConnectionRole::Shell, "0::/app.slice/sill.service\n".to_owned());
    /// assert_eq!(ask.role, ConnectionRole::Shell);
    /// ```
    pub fn new(role: ConnectionRole, caller_cgroup: String) -> ConnectAsk {
        ConnectAsk {
            role,
            caller_cgroup,
        }
    }
}

/// What the event loop answered.
#[derive(Debug)]
#[non_exhaustive]
pub enum ConnectReply {
    /// The client end of the new connection.
    Granted(OwnedFd),
    /// No connection, and why.
    Refused(ConnectRefusal),
}

/// Why a `Connect` was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ConnectRefusal {
    /// casement does not know the program that asked.
    UnknownUnit,
    /// The program that asked may not have that role.
    RoleNotAllowed,
    /// The connection could not be made.
    Unavailable,
}

impl ConnectRefusal {
    /// The D-Bus error the caller gets.
    pub fn to_error(self) -> fdo::Error {
        match self {
            ConnectRefusal::UnknownUnit => fdo::Error::AccessDenied(
                "casement does not recognise the program that asked".to_owned(),
            ),
            ConnectRefusal::RoleNotAllowed => fdo::Error::AccessDenied(
                "this program may not have that kind of connection".to_owned(),
            ),
            ConnectRefusal::Unavailable => {
                fdo::Error::Failed("casement could not make the connection".to_owned())
            }
        }
    }
}

/// The role a `Connect(role)` call names: `shell` or `agent`, nothing else.
///
/// ```
/// use casement_core::policy::ConnectionRole;
/// use casement_ipc::dbus::parse_connect_role;
///
/// assert_eq!(parse_connect_role("shell").expect("shell"), ConnectionRole::Shell);
/// assert!(parse_connect_role("public").is_err());
/// ```
pub fn parse_connect_role(text: &str) -> Result<ConnectionRole, IpcError> {
    match text {
        "shell" => Ok(ConnectionRole::Shell),
        "agent" => Ok(ConnectionRole::Agent),
        other => Err(IpcError::BadRole(other.to_owned())),
    }
}

#[zbus::interface(name = "org.quire.Compositor1")]
impl Compositor1 {
    /// `Connect(role: s) -> h`: a new Wayland connection with the role `role` (`shell` or
    /// `agent`), when the caller's unit may have it. The reply is the client end of a socket.
    async fn connect(
        &self,
        role: String,
        #[zbus(header)] header: Header<'_>,
        #[zbus(connection)] connection: &Connection,
    ) -> fdo::Result<zbus::zvariant::OwnedFd> {
        let role = parse_connect_role(&role)
            .map_err(|error| fdo::Error::InvalidArgs(error.to_string()))?;
        let sender = header
            .sender()
            .ok_or_else(|| fdo::Error::Failed("the call has no sender".to_owned()))?;
        let pid = fdo::DBusProxy::new(connection)
            .await?
            .get_connection_unix_process_id(BusName::from(sender.clone()))
            .await?;
        let caller_cgroup = self
            .cgroups
            .cgroup_of(pid)
            .map_err(|error| fdo::Error::Failed(error.to_string()))?;
        let (reply, answer) = tokio::sync::oneshot::channel();
        let call = ConnectCall {
            ask: ConnectAsk::new(role, caller_cgroup),
            reply,
        };
        self.calls
            .send(call)
            .map_err(|_| ConnectRefusal::Unavailable.to_error())?;
        match answer.await {
            Ok(ConnectReply::Granted(fd)) => Ok(zbus::zvariant::OwnedFd::from(fd)),
            Ok(ConnectReply::Refused(why)) => Err(why.to_error()),
            Err(_) => Err(ConnectRefusal::Unavailable.to_error()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_shell_and_agent_can_be_asked_for() {
        assert_eq!(
            parse_connect_role("shell").expect("shell"),
            ConnectionRole::Shell
        );
        assert_eq!(
            parse_connect_role("agent").expect("agent"),
            ConnectionRole::Agent
        );
        for text in ["", "Shell", "public", "sandboxed", "shell "] {
            assert!(parse_connect_role(text).is_err(), "{text:?}");
        }
    }

    #[test]
    fn a_refusal_is_a_dbus_error_of_the_right_kind() {
        assert!(matches!(
            ConnectRefusal::UnknownUnit.to_error(),
            fdo::Error::AccessDenied(_)
        ));
        assert!(matches!(
            ConnectRefusal::RoleNotAllowed.to_error(),
            fdo::Error::AccessDenied(_)
        ));
        assert!(matches!(
            ConnectRefusal::Unavailable.to_error(),
            fdo::Error::Failed(_)
        ));
    }
}
