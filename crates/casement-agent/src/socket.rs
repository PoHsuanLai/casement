//! The hand-out of privileged connections (C20, I7): `org.quire.Compositor1.Connect(role)`
//! answers with a connected socket whose client is inserted with that role, after checking
//! the role the caller proved. The only place a `Shell` or `Agent` connection is made (B14).
//!
//! This crate decides and makes the socket pair; the composition root inserts the server end
//! into the Wayland display as a client of the granted role (`casement::connect`), because
//! only it can build a client's data (its key, its gone queue). Frozen at P2-Z; filled by
//! P2-Q2.

use casement_core::policy::ConnectionRole;
use peephole_core::{RoleName, roles};
use std::os::fd::OwnedFd;
use std::os::unix::net::UnixStream;

/// What the caller gets.
#[derive(Debug)]
pub enum HandOut {
    /// A new connection: the server end for the display, the client end for the caller.
    Granted(Handed),
    /// Nothing, and why.
    Refused(RefuseWhy),
}

/// A granted connection: a socket pair and the role its client is inserted with.
#[derive(Debug)]
pub struct Handed {
    /// The role the client has (I7).
    pub role: ConnectionRole,
    /// The server end, for `Display::insert_client`.
    pub server: UnixStream,
    /// The client end, returned to the caller over D-Bus.
    pub client: OwnedFd,
}

/// Why a hand-out was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RefuseWhy {
    /// The caller holds no role casement hands a connection to.
    UnknownUnit,
    /// The caller's role may not have this kind of connection.
    RoleNotAllowed,
    /// The socket pair could not be made.
    NoSocket,
}

/// Hands a caller that holds `held` a connection with `wanted`, if that role is the one `held`
/// stands for: a socket pair, the role kept with it. `held` is what the bus thread proved of the
/// caller (`casement_ipc::dbus`), never anything the caller said; this check is the second line
/// should a caller of this function ask for more than the proof gave. The caller of this
/// function inserts the server end into the display.
pub fn hand_out(wanted: &ConnectionRole, held: &RoleName) -> HandOut {
    match connection_role(held) {
        None => HandOut::Refused(RefuseWhy::UnknownUnit),
        Some(allowed) if allowed != *wanted => HandOut::Refused(RefuseWhy::RoleNotAllowed),
        Some(allowed) => match UnixStream::pair().and_then(|pair| {
            // The display reads the server end from its event loop: it must never block.
            pair.0.set_nonblocking(true)?;
            Ok(pair)
        }) {
            Ok((server, client)) => HandOut::Granted(Handed {
                role: allowed,
                server,
                client: OwnedFd::from(client),
            }),
            Err(_) => HandOut::Refused(RefuseWhy::NoSocket),
        },
    }
}

/// The connection role a proven caller role stands for: the shell the shell's, the agent tier
/// the agent's, every other role none. The role comes from the shipped caller table (the
/// caller-identity library), never from a unit name.
pub fn connection_role(held: &RoleName) -> Option<ConnectionRole> {
    match held {
        held if *held == roles::SHELL => Some(ConnectionRole::Shell),
        held if *held == roles::AGENT_TIER => Some(ConnectionRole::Agent),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn role(name: &str) -> RoleName {
        RoleName::new(name).expect("a role name")
    }

    #[test]
    fn only_the_shell_and_the_agent_tier_stand_for_privileged_roles() {
        assert_eq!(connection_role(&roles::SHELL), Some(ConnectionRole::Shell));
        assert_eq!(
            connection_role(&roles::AGENT_TIER),
            Some(ConnectionRole::Agent)
        );
        for other in [
            "portal",
            "settings",
            "compositor",
            "companion",
            "shell_helper",
        ] {
            assert_eq!(connection_role(&role(other)), None, "{other}");
        }
    }

    /// (case, the role the caller proved, the role asked, what comes back)
    #[test]
    fn a_hand_out_needs_the_role_that_stands_for_the_connection() {
        let cases = [
            (
                "the shell asks for the shell",
                roles::SHELL,
                ConnectionRole::Shell,
                None,
            ),
            (
                "the agent tier asks for the agent tier",
                roles::AGENT_TIER,
                ConnectionRole::Agent,
                None,
            ),
            (
                "the shell asks for the agent tier",
                roles::SHELL,
                ConnectionRole::Agent,
                Some(RefuseWhy::RoleNotAllowed),
            ),
            (
                "the agent tier asks for the shell",
                roles::AGENT_TIER,
                ConnectionRole::Shell,
                Some(RefuseWhy::RoleNotAllowed),
            ),
            (
                "the portal asks for the shell",
                roles::PORTAL,
                ConnectionRole::Shell,
                Some(RefuseWhy::UnknownUnit),
            ),
        ];
        for (case, held, wanted, refused) in cases {
            match (hand_out(&wanted, &held), refused) {
                (HandOut::Granted(handed), None) => assert_eq!(handed.role, wanted, "{case}"),
                (HandOut::Refused(why), Some(want)) => assert_eq!(why, want, "{case}"),
                (other, _) => panic!("{case}: {other:?}"),
            }
        }
    }
}
