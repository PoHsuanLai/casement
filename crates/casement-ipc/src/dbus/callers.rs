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
/// shell, the agent tier's for the agent tier, none for any other kind of connection.
///
/// ```
/// use casement_core::policy::ConnectionRole;
/// use casement_ipc::dbus::role_for_connection;
/// use peephole_bus::peephole_core::roles;
///
/// assert_eq!(role_for_connection(&ConnectionRole::Shell), Some(roles::SHELL));
/// assert!(role_for_connection(&ConnectionRole::Public).is_none());
/// ```
pub fn role_for_connection(role: &ConnectionRole) -> Option<RoleName> {
    match role {
        ConnectionRole::Shell => Some(roles::SHELL),
        ConnectionRole::Agent => Some(roles::AGENT_TIER),
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
    let need: Vec<Requirement> = roles
        .iter()
        .map(|role| Requirement::new(role.clone(), Proof::Full))
        .collect();
    match standing.holds(&need) {
        Ok(_) => Proven::of(&standing).ok_or_else(not_the_caller),
        Err(refused) => {
            log_refusal(roles, &found(&standing, refused));
            Err(not_the_caller())
        }
    }
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
