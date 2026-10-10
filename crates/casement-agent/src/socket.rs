//! The hand-out of privileged connections (C20, I7): `org.quire.Compositor1.Connect(role)`
//! answers with a connected socket whose client is inserted with that role, after checking
//! the caller's systemd unit. The only place a `Shell` or `Agent` connection is made (B14).
//!
//! This crate decides and makes the socket pair; the composition root inserts the server end
//! into the Wayland display as a client of the granted role (`casement::connect`), because
//! only it can build a client's data (its key, its gone queue). Frozen at P2-Z; filled by
//! P2-Q2.

use casement_core::policy::ConnectionRole;
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
    /// The caller's unit is not one casement knows.
    UnknownUnit,
    /// The caller's unit may not have this role.
    RoleNotAllowed,
    /// The socket pair could not be made.
    NoSocket,
}

/// The systemd unit of the D-Bus peer that asked: `"sill.service"`, `"cuad.service"`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CallerUnit(pub String);

impl CallerUnit {
    /// The unit a process runs in, from the text of its `/proc/<pid>/cgroup`: the last path
    /// component of its cgroup when that is a service unit (`.../app.slice/sill.service`).
    /// `None` for a scope, a slice or a process outside systemd.
    ///
    /// ```
    /// use casement_agent::CallerUnit;
    ///
    /// let unit = CallerUnit::of_cgroup("0::/user.slice/app.slice/sill.service\n");
    /// assert_eq!(unit, Some(CallerUnit("sill.service".to_owned())));
    /// assert_eq!(CallerUnit::of_cgroup("0::/\n"), None);
    /// ```
    pub fn of_cgroup(text: &str) -> Option<CallerUnit> {
        let path = text.lines().find_map(|line| line.strip_prefix("0::"))?;
        let last = path.rsplit('/').next()?;
        let named = last
            .strip_suffix(".service")
            .is_some_and(|name| !name.is_empty());
        named.then(|| CallerUnit(last.to_owned()))
    }
}

/// Hands `caller` a connection with `role` if its unit may have it: a socket pair, the role
/// kept with it. The caller of this function inserts the server end into the display.
pub fn hand_out(role: &ConnectionRole, caller: &CallerUnit) -> HandOut {
    match role_for(caller) {
        None => HandOut::Refused(RefuseWhy::UnknownUnit),
        Some(allowed) if allowed != *role => HandOut::Refused(RefuseWhy::RoleNotAllowed),
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

/// The role `caller`'s unit may have: `sill.service` the shell, `cuad.service` the agent tier,
/// anything else none.
pub fn role_for(caller: &CallerUnit) -> Option<ConnectionRole> {
    match caller.0.as_str() {
        "sill.service" => Some(ConnectionRole::Shell),
        "cuad.service" => Some(ConnectionRole::Agent),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit(name: &str) -> CallerUnit {
        CallerUnit(name.to_owned())
    }

    #[test]
    fn only_the_shell_and_the_agent_tier_get_privileged_roles() {
        assert_eq!(role_for(&unit("sill.service")), Some(ConnectionRole::Shell));
        assert_eq!(role_for(&unit("cuad.service")), Some(ConnectionRole::Agent));
        assert_eq!(role_for(&unit("firefox.service")), None);
        assert_eq!(role_for(&unit("")), None);
    }

    /// (case, who asks, the role asked, what comes back)
    #[test]
    fn a_hand_out_needs_the_unit_that_may_have_the_role() {
        let cases = [
            (
                "sill asks for the shell",
                "sill.service",
                ConnectionRole::Shell,
                None,
            ),
            (
                "cuad asks for the agent tier",
                "cuad.service",
                ConnectionRole::Agent,
                None,
            ),
            (
                "sill asks for the agent tier",
                "sill.service",
                ConnectionRole::Agent,
                Some(RefuseWhy::RoleNotAllowed),
            ),
            (
                "cuad asks for the shell",
                "cuad.service",
                ConnectionRole::Shell,
                Some(RefuseWhy::RoleNotAllowed),
            ),
            (
                "a browser asks for the shell",
                "firefox.service",
                ConnectionRole::Shell,
                Some(RefuseWhy::UnknownUnit),
            ),
            (
                "an app scope asks for the shell",
                "app-org.quire.Mail-12.scope",
                ConnectionRole::Shell,
                Some(RefuseWhy::UnknownUnit),
            ),
        ];
        for (case, who, role, refused) in cases {
            match (hand_out(&role, &unit(who)), refused) {
                (HandOut::Granted(handed), None) => assert_eq!(handed.role, role, "{case}"),
                (HandOut::Refused(why), Some(want)) => assert_eq!(why, want, "{case}"),
                (other, _) => panic!("{case}: {other:?}"),
            }
        }
    }

    /// (case, the text of /proc/<pid>/cgroup, the unit)
    #[test]
    fn a_unit_is_read_from_the_cgroup_text() {
        let cases = [
            (
                "a user service",
                "0::/user.slice/user-1000.slice/user@1000.service/app.slice/sill.service\n",
                Some("sill.service"),
            ),
            (
                "an app scope is not a service",
                "0::/user.slice/user-1000.slice/user@1000.service/app.slice/app-org.quire.Mail-12.scope\n",
                None,
            ),
            ("not under systemd", "0::/\n", None),
            ("empty", "", None),
        ];
        for (case, text, want) in cases {
            assert_eq!(CallerUnit::of_cgroup(text), want.map(unit), "{case}");
        }
    }
}
