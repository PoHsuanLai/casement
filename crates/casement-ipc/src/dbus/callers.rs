//! Who is calling `org.quire.Compositor1`, proven on the bus thread before a call reaches the
//! event loop (CASW-05). The proof is the caller-identity library's: the sender's pinned
//! process, its image, and its unit as the unit manager states it, judged against the shipped
//! caller table. A cgroup name alone proves nothing, so none is read here.

use super::connect_ask::{NOT_THE_CALLER, Proven};
use casement_core::policy::ConnectionRole;
use peephole_bus::peephole::ProcFs;
use peephole_bus::peephole::RealProc;
use peephole_bus::peephole_core::{Proof, Refused, Requirement, RoleName, Standing, roles};
use peephole_bus::{BusPeers, ManagerLink, Offload, Tokio, UnitSource, WhoError};
use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;
use zbus::fdo;

/// The standing of a sender, as a future the bus thread awaits.
pub type WhoFuture<'a> = Pin<Box<dyn Future<Output = Result<Standing, WhoError>> + Send + 'a>>;

/// The seam that says who a bus sender is. The real one is [`BusPeers`]; a test makes a sender
/// stand as a chosen caller (`BusPeers::introduce`), and a test-only build can name units for
/// jailed peers (`DebugPeerUnits`).
pub trait CallerCheck: Debug + Send + Sync + 'static {
    /// The standing of the unique bus name `sender`. An error is an unknown caller.
    fn who<'a>(&'a self, sender: &'a str) -> WhoFuture<'a>;
}

impl<P, O, U> CallerCheck for BusPeers<P, O, U>
where
    P: ProcFs + Debug + 'static,
    O: Offload + Debug + 'static,
    U: UnitSource + Debug + 'static,
{
    fn who<'a>(&'a self, sender: &'a str) -> WhoFuture<'a> {
        Box::pin(BusPeers::who(self, sender))
    }
}

/// The real identifier: the system's `/proc`, the unit manager on the bus, the probe run on the
/// blocking pool of the tokio runtime the bus thread owns.
pub type DesktopCallers = BusPeers<RealProc, Tokio, ManagerLink<RealProc>>;

/// The role a caller must hold to be handed a connection with `role`: the shell's for the
/// shell, the agent tier's for the agent tier, the portal's for the portal, none for any other
/// kind of connection.
///
/// ```
/// use casement_core::policy::ConnectionRole;
/// use casement_ipc::dbus::role_for_connection;
/// use peephole_bus::peephole_core::roles;
///
/// assert_eq!(role_for_connection(&ConnectionRole::Shell), Some(roles::SHELL));
/// assert_eq!(role_for_connection(&ConnectionRole::Portal), Some(roles::PORTAL));
/// assert!(role_for_connection(&ConnectionRole::Public).is_none());
/// ```
pub fn role_for_connection(role: &ConnectionRole) -> Option<RoleName> {
    match role {
        ConnectionRole::Shell => Some(roles::SHELL),
        ConnectionRole::Agent => Some(roles::AGENT_TIER),
        ConnectionRole::Portal => Some(roles::PORTAL),
        _ => None,
    }
}

/// Proves that `sender` holds one of `roles`, all the way: the image and the unit as the manager
/// states it (`Proof::Full`), with either kind of pin (a bus may not offer a process
/// descriptor). A caller that is not one of them is refused with the one fixed sentence, and the
/// refusal is logged once with the roles asked and the standing found.
pub async fn require(
    check: &dyn CallerCheck,
    sender: Option<&str>,
    roles: &[RoleName],
) -> fdo::Result<Proven> {
    let Some(sender) = sender else {
        log_refusal(roles, &"no sender");
        return Err(not_the_caller());
    };
    let standing = match check.who(sender).await {
        Ok(standing) => standing,
        Err(error) => {
            log_refusal(roles, &error);
            return Err(not_the_caller());
        }
    };
    match admitted(&standing, roles) {
        Ok(()) => Proven::of(&standing).ok_or_else(not_the_caller),
        Err(refused) => {
            log_refusal(roles, &found(&standing, refused));
            Err(not_the_caller())
        }
    }
}

/// Whether `standing` holds one of `roles` all the way (`Proof::Full`) and, only in a build with
/// feature `dev-rows`, as a developer row (`Proof::Dev`) of one of them. The developer step is
/// exactly that proof, so a service known by its image or its control group alone is refused
/// there as it is everywhere.
fn admitted(standing: &Standing, roles: &[RoleName]) -> Result<(), Refused> {
    let need: Vec<Requirement> = roles
        .iter()
        .map(|role| Requirement::new(role.clone(), Proof::Full))
        .collect();
    let refused = match standing.holds(&need) {
        Ok(_) => return Ok(()),
        Err(refused) => refused,
    };
    match dev_row_of(standing, roles) {
        true => Ok(()),
        false => Err(refused),
    }
}

/// Whether `standing` is a developer row's service of one of `roles`.
#[cfg(feature = "dev-rows")]
fn dev_row_of(standing: &Standing, roles: &[RoleName]) -> bool {
    matches!(
        standing,
        Standing::Service { role, proof: Proof::Dev, .. } if roles.contains(role)
    )
}

/// No developer rows in this build: nothing is one.
#[cfg(not(feature = "dev-rows"))]
fn dev_row_of(_standing: &Standing, _roles: &[RoleName]) -> bool {
    false
}

/// What the log says was found: the standing and why it did not do.
fn found(standing: &Standing, refused: Refused) -> String {
    format!("{standing:?} ({refused})")
}

fn log_refusal(roles: &[RoleName], found: &dyn std::fmt::Display) {
    tracing::warn!(
        asked = ?roles,
        found = %found,
        "a call to casement over the bus was refused: the caller does not hold a role the method answers"
    );
}

fn not_the_caller() -> fdo::Error {
    fdo::Error::AccessDenied(NOT_THE_CALLER.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use peephole_bus::peephole_core::{AppKey, Pin as Pinning};

    fn shell_at(proof: Proof) -> Standing {
        Standing::Service {
            app: AppKey::new("org.quire.Shell").expect("an app"),
            role: roles::SHELL,
            proof,
            pin: Pinning::Reopened,
        }
    }

    #[test]
    fn a_role_is_held_all_the_way_or_not_at_all() {
        let asked = [roles::SHELL];
        assert!(admitted(&shell_at(Proof::Full), &asked).is_ok());
        assert!(admitted(&shell_at(Proof::Exe), &asked).is_err());
        assert!(admitted(&shell_at(Proof::Cgroup), &asked).is_err());
        assert!(admitted(&shell_at(Proof::Full), &[roles::PORTAL]).is_err());
    }

    /// A developer row's service is let in for its own role and only that proof; the other
    /// refusals stand. A build without `dev-rows` has no such proof to be let in.
    #[cfg(feature = "dev-rows")]
    #[test]
    fn a_developer_row_is_let_in_for_its_own_role_only() {
        assert!(admitted(&shell_at(Proof::Dev), &[roles::SHELL]).is_ok());
        assert!(admitted(&shell_at(Proof::Dev), &[roles::AGENT_TIER, roles::SHELL]).is_ok());
        assert!(admitted(&shell_at(Proof::Dev), &[roles::PORTAL]).is_err());
        assert!(admitted(&shell_at(Proof::Exe), &[roles::SHELL]).is_err());
        assert!(admitted(&shell_at(Proof::Cgroup), &[roles::SHELL]).is_err());
    }

    #[cfg(not(feature = "dev-rows"))]
    #[test]
    fn no_standing_is_a_developer_row_in_a_build_without_them() {
        assert!(!dev_row_of(&shell_at(Proof::Full), &[roles::SHELL]));
    }
}
