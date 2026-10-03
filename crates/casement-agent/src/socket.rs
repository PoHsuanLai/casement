//! The hand-out of privileged connections (C20, I7): `org.quire.Compositor1.Connect(role)`
//! answers with a connected socket whose client is inserted with that role, after checking
//! the caller's systemd unit. The only place a `Shell` or `Agent` connection is made (B14).

use casement_core::policy::ConnectionRole;
use smithay::reexports::wayland_server::DisplayHandle;
use std::os::fd::OwnedFd;

/// What the caller gets.
#[derive(Debug)]
pub enum HandOut {
    /// The client end of a new connection with the asked role.
    Granted(OwnedFd),
    /// Nothing, and why.
    Refused(RefuseWhy),
}

/// Why a hand-out was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RefuseWhy {
    /// The caller's unit is not one casement knows.
    UnknownUnit,
    /// The caller's unit may not have this role.
    RoleNotAllowed,
}

/// The systemd unit of the D-Bus peer that asked: `"sill.service"`, `"cuad.service"`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CallerUnit(pub String);

/// Hands `caller` a connection with `role` if its unit may have it: a socket pair, the server
/// end inserted into `display` with that role, the client end returned.
pub fn hand_out(
    display: &mut DisplayHandle,
    role: &ConnectionRole,
    caller: &CallerUnit,
) -> HandOut {
    let _ = (display, role, caller);
    todo!("P2-Q2: hand_out")
}

/// The role `caller`'s unit may have: `sill.service` the shell, `cuad.service` the agent tier,
/// anything else none.
pub fn role_for(caller: &CallerUnit) -> Option<ConnectionRole> {
    let _ = caller;
    todo!("P2-Q2: role_for")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "P2-Q2"]
    fn only_the_shell_and_the_agent_tier_get_privileged_roles() {
        let unit = |name: &str| CallerUnit(name.to_owned());
        assert_eq!(role_for(&unit("sill.service")), Some(ConnectionRole::Shell));
        assert_eq!(role_for(&unit("cuad.service")), Some(ConnectionRole::Agent));
        assert_eq!(role_for(&unit("firefox.service")), None);
        assert_eq!(role_for(&unit("")), None);
    }
}
