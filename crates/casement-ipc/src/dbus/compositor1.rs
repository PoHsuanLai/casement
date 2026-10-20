//! `org.quire.Compositor1` at `/org/quire/Compositor1` (the session bus; casement owns the
//! name): the way the shell, the agent tier and the portal get their privileged Wayland
//! connections.
//!
//! ```text
//! Connect(role: s) -> (h)    role is "shell", "agent" or "portal"; the reply is the client end
//!                            of a new connection that has that role (I7)
//! ConnectEis(session: t, devices: u) -> (h)
//!                            only the desktop portal may ask: the reply is the client end of
//!                            an EIS socket whose input arrives tagged as remote input of
//!                            portal session `session`; `devices` is the portal's `types` bit
//!                            set (1 keyboard, 2 pointer, 4 touchscreen)
//! EndEis(session: t)         only the desktop portal may ask: input from every socket handed
//!                            out for `session` stops (keys it still holds are let go) and the
//!                            compositor's end of each is closed; a session it does not know is
//!                            not an error
//! LeaseWindow(window: s, devices: u) -> (t, h)
//!                            only the agent tier may ask: a lease on the window whose
//!                            ext-foreign-toplevel identifier is `window`, and the client end of
//!                            an EIS socket whose input reaches that window and its popups and
//!                            nothing else (never another window, a layer surface, the lock or a
//!                            trusted surface, and never a compositor binding); the lease ends
//!                            with the window, with `EndLease`, or when the caller's bus
//!                            connection goes
//! EndLease(lease: t)         only the agent tier may ask: the lease's input stops (keys it
//!                            still holds are let go) and its socket closes; a lease it does not
//!                            know is not an error
//! ```
//!
//! Who asks is decided on the bus thread, before anything reaches the event loop (CASW-05):
//! a caller is counted against its rate, proven to hold the role the method answers (the shell
//! for the shell, the agent tier for the agent tier, the portal for its connection and its two
//! EIS calls) through
//! the caller-identity library, and only then is the question put in a queue of
//! [`CONNECT_QUEUE`](super::CONNECT_QUEUE) for casement's event loop, which makes the socket pair and inserts the
//! server end into the display (`casement_agent::hand_out`). A call beyond the rate or the
//! queue is dropped with a plain error. P4-Z adds `Desktop() -> s`, `Perform(action: s) -> s`
//! and the `Killed()` signal. Filled by P2-Q2.

use super::callers::{CallerCheck, require, role_for_connection};
use super::connect_ask::{ConnectAsk, ConnectRefusal, ConnectReply, Proven};
use super::intake::{ANSWER_WAIT, Intake};
use super::portal_watch::{PortalWatch, Watching};
use crate::IpcError;
use casement_core::policy::ConnectionRole;
use peephole_bus::peephole_core::{RoleName, roles};
use std::sync::Arc;
use zbus::fdo;
use zbus::message::Header;

/// The object's path.
pub const OBJECT_PATH: &str = "/org/quire/Compositor1";

/// The well-known name casement owns on the session bus.
pub const BUS_NAME: &str = "org.quire.Compositor1";

/// The object: the door into the event loop, where every call is carried out, and the check
/// that says who a caller is.
#[derive(Debug)]
pub struct Compositor1 {
    intake: Arc<Intake>,
    callers: Arc<dyn CallerCheck>,
    portals: Arc<PortalWatch>,
}

impl Compositor1 {
    /// The object over `intake`, proving callers through `callers`, remembering which bus
    /// connections asked for remote sockets in `portals`.
    pub fn new(
        intake: Arc<Intake>,
        callers: Arc<dyn CallerCheck>,
        portals: Arc<PortalWatch>,
    ) -> Compositor1 {
        Compositor1 {
            intake,
            callers,
            portals,
        }
    }
}

/// The role a `Connect(role)` call names: `shell`, `agent` or `portal`, nothing else.
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
        "portal" => Ok(ConnectionRole::Portal),
        other => Err(IpcError::BadRole(other.to_owned())),
    }
}

#[zbus::interface(name = "org.quire.Compositor1")]
impl Compositor1 {
    /// `Connect(role: s) -> h`: a new Wayland connection with the role `role` (`shell`, `agent`
    /// or `portal`), when the caller holds the role that stands for it. The reply is the client end
    /// of a socket.
    async fn connect(
        &self,
        role: String,
        #[zbus(header)] header: Header<'_>,
    ) -> fdo::Result<zbus::zvariant::OwnedFd> {
        let role = parse_connect_role(&role)
            .map_err(|error| fdo::Error::InvalidArgs(error.to_string()))?;
        let needed =
            role_for_connection(&role).ok_or_else(|| ConnectRefusal::NotTheCaller.to_error())?;
        let caller = self.proven(&header, &needed).await?;
        self.ask(ConnectAsk::new(role, caller)).await
    }

    /// `ConnectEis(session: t, devices: u) -> h`: an EIS socket for the RemoteDesktop portal
    /// session `session`, offering the device kinds in `devices` (1 keyboard, 2 pointer, 4
    /// touchscreen). Only the portal may ask. Input sent down the socket reaches the desktop
    /// tagged as remote.
    async fn connect_eis(
        &self,
        session: u64,
        devices: u32,
        #[zbus(header)] header: Header<'_>,
    ) -> fdo::Result<zbus::zvariant::OwnedFd> {
        let caller = self.proven(&header, &roles::PORTAL).await?;
        // The socket belongs to the portal's connection to the bus, not to the portal as a
        // program: when that connection goes (a restart, a crash), so do its sockets.
        let owner = self.watched_owner(&header, &caller)?;
        self.ask(ConnectAsk::remote_eis(session, devices, caller).from_owner(&owner))
            .await
    }

    /// `EndEis(session: t)`: input from every socket handed out for portal session `session`
    /// stops and the compositor's end of each is closed. Only the portal may ask, as for
    /// `ConnectEis`.
    async fn end_eis(&self, session: u64, #[zbus(header)] header: Header<'_>) -> fdo::Result<()> {
        let caller = self.proven(&header, &roles::PORTAL).await?;
        match self.answer(ConnectAsk::end_eis(session, caller)).await? {
            ConnectReply::Ended => Ok(()),
            ConnectReply::Refused(why) => Err(why.to_error()),
            _ => Err(ConnectRefusal::Unavailable.to_error()),
        }
    }

    /// `LeaseWindow(window: s, devices: u) -> (t, h)`: a lease on the window whose
    /// ext-foreign-toplevel identifier is `window` and the client end of its input socket,
    /// offering the device kinds in `devices` (1 keyboard, 2 pointer, 4 touchscreen). Only the
    /// agent tier may ask. Input sent down the socket reaches that window and its popups only.
    async fn lease_window(
        &self,
        window: String,
        devices: u32,
        #[zbus(header)] header: Header<'_>,
    ) -> fdo::Result<(u64, zbus::zvariant::OwnedFd)> {
        let caller = self.proven(&header, &roles::AGENT_TIER).await?;
        let owner = sender_of(&header)?;
        let ask = ConnectAsk::lease_window(&window, devices, caller.clone()).from_owner(&owner);
        let (lease, socket) = match self.answer(ask).await? {
            ConnectReply::Leased { lease, socket } => (lease, socket),
            ConnectReply::Refused(why) => return Err(why.to_error()),
            _ => return Err(ConnectRefusal::Unavailable.to_error()),
        };
        // The lease belongs to the agent tier's bus connection: when it goes, so does the lease.
        // It is watched only once a lease was granted, so a refused ask takes no room.
        if self.portals.watch(&owner, &caller) == Watching::TableFull {
            let _ = self
                .answer(ConnectAsk::end_lease(lease, caller).from_owner(&owner))
                .await;
            return Err(ConnectRefusal::TooMany.to_error());
        }
        Ok((lease, zbus::zvariant::OwnedFd::from(socket)))
    }

    /// `EndLease(lease: t)`: the lease's input stops and its socket closes. Only the agent tier
    /// may ask, as for `LeaseWindow`.
    async fn end_lease(&self, lease: u64, #[zbus(header)] header: Header<'_>) -> fdo::Result<()> {
        let caller = self.proven(&header, &roles::AGENT_TIER).await?;
        let owner = sender_of(&header)?;
        match self
            .answer(ConnectAsk::end_lease(lease, caller).from_owner(&owner))
            .await?
        {
            ConnectReply::Ended => Ok(()),
            ConnectReply::Refused(why) => Err(why.to_error()),
            _ => Err(ConnectRefusal::Unavailable.to_error()),
        }
    }
}

/// The unique bus name of the connection that sent `header`.
fn sender_of(header: &Header<'_>) -> fdo::Result<String> {
    header
        .sender()
        .map(|name| name.as_str().to_owned())
        .ok_or_else(|| ConnectRefusal::NotTheCaller.to_error())
}

impl Compositor1 {
    /// The unique bus name of the connection that sent `header`, now watched so what it asked
    /// for ends when it leaves the bus.
    fn watched_owner(&self, header: &Header<'_>, caller: &Proven) -> fdo::Result<String> {
        let owner = sender_of(header)?;
        match self.portals.watch(&owner, caller) {
            Watching::Yes => Ok(owner),
            Watching::TableFull => Err(ConnectRefusal::TooMany.to_error()),
        }
    }

    /// Counts the call against its caller's rate, then proves the caller holds `needed`.
    async fn proven(&self, header: &Header<'_>, needed: &RoleName) -> fdo::Result<Proven> {
        let sender = header.sender().map(zbus::names::UniqueName::as_str);
        if let Some(name) = sender {
            self.intake.rate(name).map_err(ConnectRefusal::to_error)?;
        }
        require(&*self.callers, sender, std::slice::from_ref(needed)).await
    }

    /// Puts `ask` in the queue and returns the event loop's answer; a loop that is gone or does
    /// not answer in [`ANSWER_WAIT`] is `Unavailable`, a full queue drops the call.
    async fn answer(&self, ask: ConnectAsk) -> fdo::Result<ConnectReply> {
        let answer = self.intake.enqueue(ask).map_err(ConnectRefusal::to_error)?;
        match tokio::time::timeout(ANSWER_WAIT, answer).await {
            Ok(Ok(reply)) => Ok(reply),
            Ok(Err(_)) | Err(_) => Err(ConnectRefusal::Unavailable.to_error()),
        }
    }

    /// Hands `ask` to the event loop and turns its answer into the reply or a D-Bus error.
    async fn ask(&self, ask: ConnectAsk) -> fdo::Result<zbus::zvariant::OwnedFd> {
        match self.answer(ask).await? {
            ConnectReply::Granted(fd) => Ok(zbus::zvariant::OwnedFd::from(fd)),
            ConnectReply::Refused(why) => Err(why.to_error()),
            _ => Err(ConnectRefusal::Unavailable.to_error()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_shell_agent_and_portal_can_be_asked_for() {
        assert_eq!(
            parse_connect_role("shell").expect("shell"),
            ConnectionRole::Shell
        );
        assert_eq!(
            parse_connect_role("agent").expect("agent"),
            ConnectionRole::Agent
        );
        assert_eq!(
            parse_connect_role("portal").expect("portal"),
            ConnectionRole::Portal
        );
        for text in ["", "Shell", "public", "sandboxed", "shell "] {
            assert!(parse_connect_role(text).is_err(), "{text:?}");
        }
    }

    #[test]
    fn a_refusal_is_a_dbus_error_of_the_right_kind() {
        for denied in [
            ConnectRefusal::UnknownUnit,
            ConnectRefusal::RoleNotAllowed,
            ConnectRefusal::NotTheCaller,
        ] {
            assert!(matches!(denied.to_error(), fdo::Error::AccessDenied(_)));
        }
        assert!(matches!(
            ConnectRefusal::TooMany.to_error(),
            fdo::Error::LimitsExceeded(_)
        ));
        assert!(matches!(
            ConnectRefusal::Unavailable.to_error(),
            fdo::Error::Failed(_)
        ));
    }
}
