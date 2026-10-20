//! What crosses from the bus thread to casement's event loop for `org.quire.Compositor1`: the
//! question ([`ConnectAsk`], with the caller already proven), where the answer goes
//! ([`ConnectCall`]) and the answer ([`ConnectReply`], [`ConnectRefusal`]).

use casement_core::policy::ConnectionRole;
use peephole_bus::peephole_core::{AppKey, RoleName, Standing};
use std::os::fd::OwnedFd;
use zbus::fdo;

/// The one sentence a refused caller is told, whatever check failed: it must not say which one
/// (a caller learns nothing about the table or the proofs from the refusal).
pub const NOT_THE_CALLER: &str = "not the caller this method answers";

/// Who the caller proved to be on the bus thread: the app and the role of the table row it
/// matched, at the proof the call needed. Never read from the caller's own words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proven {
    /// The row's app.
    pub app: AppKey,
    /// The row's role.
    pub role: RoleName,
}

impl Proven {
    /// The caller `app` holding `role`.
    ///
    /// ```
    /// use casement_ipc::dbus::Proven;
    /// use peephole_bus::peephole_core::{AppKey, roles};
    ///
    /// let shell = Proven::new(AppKey::new("org.quire.Shell").expect("an app"), roles::SHELL);
    /// assert_eq!(shell.role, roles::SHELL);
    /// ```
    pub fn new(app: AppKey, role: RoleName) -> Proven {
        Proven { app, role }
    }

    /// The app and role of a table row's standing; nothing for any other standing.
    pub fn of(standing: &Standing) -> Option<Proven> {
        match standing {
            Standing::Service { app, role, .. } => Some(Proven::new(app.clone(), role.clone())),
            _ => None,
        }
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

/// What a caller asked for, once the bus thread has checked who it is.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ConnectAsk {
    /// The role it wants a connection to have.
    pub role: ConnectionRole,
    /// Who the caller proved to be.
    pub caller: Proven,
    /// Set for `ConnectEis`: the question is for a remote EIS socket, not a Wayland
    /// connection, and `role` is not read.
    pub remote: Option<RemoteAsk>,
    /// Set for `EndEis`: the question is to end a remote session, and neither `role` nor
    /// `remote` is read.
    pub end: Option<EndAsk>,
    /// Set for `ConnectEis`: the unique bus name of the portal connection that asked, so its
    /// sockets can be ended when that connection goes (NEW-PI-8).
    pub owner: Option<String>,
    /// Set when a portal connection left the bus (made by casement's own bus thread, never by a
    /// caller): the question is to end every remote socket that connection asked for.
    pub gone: Option<GoneAsk>,
    /// Set for `LeaseWindow`: the question is for a lease on one window and the input socket
    /// of that lease; `role` is not read.
    pub lease: Option<LeaseAsk>,
    /// Set for `EndLease`: the question is to end a lease.
    pub end_lease: Option<EndLeaseAsk>,
}

/// What the agent tier asked of `LeaseWindow`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaseAsk {
    /// The window, by the identifier its `ext_foreign_toplevel_handle_v1` carries.
    pub window: String,
    /// The device kinds, as a bit set (1 keyboard, 2 pointer, 4 touchscreen).
    pub devices: u32,
}

/// What the agent tier asked of `EndLease`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EndLeaseAsk {
    /// The lease to end.
    pub lease: u64,
}

/// A portal connection that left the bus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoneAsk {
    /// Its unique bus name.
    pub owner: String,
}

/// What the portal asked of `EndEis`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EndAsk {
    /// The portal session whose sockets are to be closed.
    pub session: u64,
}

/// What the portal asked of `ConnectEis`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoteAsk {
    /// The portal session the input belongs to.
    pub session: u64,
    /// The device kinds, as the portal's `types` bit set.
    pub devices: u32,
}

impl ConnectAsk {
    /// The question of `caller`, asking for a connection with `role`.
    ///
    /// ```
    /// use casement_core::policy::ConnectionRole;
    /// use casement_ipc::dbus::{ConnectAsk, Proven};
    /// use peephole_bus::peephole_core::{AppKey, roles};
    ///
    /// let caller = Proven::new(AppKey::new("org.quire.Shell").expect("an app"), roles::SHELL);
    /// let ask = ConnectAsk::new(ConnectionRole::Shell, caller);
    /// assert_eq!(ask.role, ConnectionRole::Shell);
    /// ```
    pub fn new(role: ConnectionRole, caller: Proven) -> ConnectAsk {
        ConnectAsk {
            role,
            caller,
            remote: None,
            end: None,
            owner: None,
            gone: None,
            lease: None,
            end_lease: None,
        }
    }

    /// The same question, asked from the bus connection `owner` (its unique name).
    #[must_use]
    pub fn from_owner(self, owner: &str) -> ConnectAsk {
        ConnectAsk {
            owner: Some(owner.to_owned()),
            ..self
        }
    }

    /// The portal connection `owner` left the bus; `portal` is who it had proven to be. Only
    /// casement's bus thread makes this.
    pub fn owner_gone(owner: &str, portal: Proven) -> ConnectAsk {
        ConnectAsk {
            // Not read: `gone` decides.
            role: ConnectionRole::Public,
            caller: portal,
            remote: None,
            end: None,
            owner: None,
            gone: Some(GoneAsk {
                owner: owner.to_owned(),
            }),
            lease: None,
            end_lease: None,
        }
    }

    /// The question of `caller`, asking for a remote EIS socket for portal session `session`
    /// with the `devices` bit set.
    ///
    /// ```
    /// use casement_ipc::dbus::{ConnectAsk, Proven};
    /// use peephole_bus::peephole_core::{AppKey, roles};
    ///
    /// let caller = Proven::new(AppKey::new("org.quire.Portal").expect("an app"), roles::PORTAL);
    /// let ask = ConnectAsk::remote_eis(7, 3, caller);
    /// assert_eq!(ask.remote.map(|remote| remote.session), Some(7));
    /// ```
    pub fn remote_eis(session: u64, devices: u32, caller: Proven) -> ConnectAsk {
        ConnectAsk {
            // Not read: `remote` decides.
            role: ConnectionRole::Public,
            caller,
            remote: Some(RemoteAsk { session, devices }),
            end: None,
            owner: None,
            gone: None,
            lease: None,
            end_lease: None,
        }
    }

    /// The question of `caller`, asking to end portal session `session`'s remote sockets.
    ///
    /// ```
    /// use casement_ipc::dbus::{ConnectAsk, Proven};
    /// use peephole_bus::peephole_core::{AppKey, roles};
    ///
    /// let caller = Proven::new(AppKey::new("org.quire.Portal").expect("an app"), roles::PORTAL);
    /// let ask = ConnectAsk::end_eis(7, caller);
    /// assert_eq!(ask.end.map(|end| end.session), Some(7));
    /// ```
    pub fn end_eis(session: u64, caller: Proven) -> ConnectAsk {
        ConnectAsk {
            // Not read: `end` decides.
            role: ConnectionRole::Public,
            caller,
            remote: None,
            end: Some(EndAsk { session }),
            owner: None,
            gone: None,
            lease: None,
            end_lease: None,
        }
    }
}

impl ConnectAsk {
    /// The question of `caller`, asking for a lease on the window whose foreign-toplevel
    /// identifier is `window`, with an input socket offering the `devices` bit set.
    ///
    /// ```
    /// use casement_ipc::dbus::{ConnectAsk, Proven};
    /// use peephole_bus::peephole_core::{AppKey, roles};
    ///
    /// let caller = Proven::new(AppKey::new("org.quire.Cua").expect("an app"), roles::AGENT_TIER);
    /// let ask = ConnectAsk::lease_window("w7", 7, caller);
    /// assert_eq!(ask.lease.map(|lease| lease.window), Some("w7".to_owned()));
    /// ```
    pub fn lease_window(window: &str, devices: u32, caller: Proven) -> ConnectAsk {
        ConnectAsk {
            lease: Some(LeaseAsk {
                window: window.to_owned(),
                devices,
            }),
            // Not read: `lease` decides.
            ..ConnectAsk::new(ConnectionRole::Public, caller)
        }
    }

    /// The question of `caller`, asking to end `lease`.
    pub fn end_lease(lease: u64, caller: Proven) -> ConnectAsk {
        ConnectAsk {
            end_lease: Some(EndLeaseAsk { lease }),
            // Not read: `end_lease` decides.
            ..ConnectAsk::new(ConnectionRole::Public, caller)
        }
    }
}

/// What the event loop answered.
#[derive(Debug)]
#[non_exhaustive]
pub enum ConnectReply {
    /// The client end of the new connection.
    Granted(OwnedFd),
    /// The remote session's sockets are closed (`EndEis`, `EndLease`).
    Ended,
    /// The lease's number and the client end of its input socket (`LeaseWindow`).
    Leased {
        /// The lease.
        lease: u64,
        /// The client end of the socket.
        socket: OwnedFd,
    },
    /// No connection, and why.
    Refused(ConnectRefusal),
}

/// Why a `Connect` was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ConnectRefusal {
    /// The caller holds no role casement hands a connection to (the event loop's second check).
    UnknownUnit,
    /// The caller's role may not have that kind of connection.
    RoleNotAllowed,
    /// The caller is not the one this method answers (the bus thread's check).
    NotTheCaller,
    /// Too many calls: this caller asked again too soon, or casement has as many as it will
    /// take at once.
    TooMany,
    /// The connection could not be made.
    Unavailable,
    /// No window has that identifier (`LeaseWindow`).
    NoSuchWindow,
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
            ConnectRefusal::NotTheCaller => fdo::Error::AccessDenied(NOT_THE_CALLER.to_owned()),
            ConnectRefusal::TooMany => fdo::Error::LimitsExceeded(
                "casement is answering other calls; ask again in a moment".to_owned(),
            ),
            ConnectRefusal::Unavailable => {
                fdo::Error::Failed("casement could not make the connection".to_owned())
            }
            ConnectRefusal::NoSuchWindow => {
                fdo::Error::InvalidArgs("no window has that identifier".to_owned())
            }
        }
    }
}
