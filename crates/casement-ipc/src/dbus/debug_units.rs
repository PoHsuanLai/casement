//! Test unit overrides (feature `debug-inject` only): a table that says "the peer running the
//! program `sill` is unit `sill.service`", for a jailed desktop scenario that has no systemd and
//! so no unit manager to vouch for a unit. [`DebugPeerUnits`] is a [`CallerCheck`] that makes a
//! matching peer stand as the role of the unit the table names, and asks the real check about
//! every other. This file is compiled only under `debug-inject`; a release build has no such
//! type, and nothing in it reads the environment (the binary parses its variable and hands the
//! text to [`PeerUnitTable::parse`]).

use super::callers::{CallerCheck, WhoFuture};
use peephole_bus::peephole_core::{AppKey, Pin as Pinning, Proof, RoleName, Standing, roles};
use std::sync::Arc;
use zbus::Connection;
use zbus::fdo::DBusProxy;
use zbus::names::BusName;

/// Who an override applies to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PeerMatch {
    /// The peer whose `/proc/<pid>/comm` is this name (the kernel keeps 15 bytes, so a longer
    /// name here is cut to 15 before it is compared).
    Exe(String),
    /// The peer with exactly this process id.
    Pid(u32),
}

/// One override: peers matching `who` run in `unit`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerUnitRule {
    /// Which peers.
    pub who: PeerMatch,
    /// The unit they are taken to run in, e.g. `sill.service`.
    pub unit: String,
}

/// Why a table did not parse.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum PeerUnitsError {
    /// An entry had no `=`, or an empty side.
    #[error("not <program-or-pid>=<unit>: {0:?}")]
    BadEntry(String),
    /// A unit that is not a service name (`name.service`, no slash).
    #[error("not a service unit name: {0:?}")]
    BadUnit(String),
}

/// The overrides, first match wins.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PeerUnitTable {
    rules: Vec<PeerUnitRule>,
}

impl PeerUnitTable {
    /// Parses `"<program-or-pid>=<unit>[,...]"`: an all-digit left side is a process id, anything
    /// else a program name. Empty text is an empty table.
    ///
    /// ```
    /// use casement_ipc::dbus::{PeerMatch, PeerUnitTable};
    ///
    /// let table = PeerUnitTable::parse("sill=sill.service,4242=cuad.service").unwrap();
    /// assert_eq!(table.rules().len(), 2);
    /// assert_eq!(table.rules()[0].who, PeerMatch::Exe("sill".to_owned()));
    /// assert_eq!(table.rules()[1].who, PeerMatch::Pid(4242));
    /// ```
    pub fn parse(text: &str) -> Result<PeerUnitTable, PeerUnitsError> {
        let rules = text
            .split(',')
            .map(str::trim)
            .filter(|entry| !entry.is_empty())
            .map(parse_rule)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(PeerUnitTable { rules })
    }

    /// The rules, in the order they were written.
    pub fn rules(&self) -> &[PeerUnitRule] {
        &self.rules
    }

    /// The unit for a peer with this pid and program name, if a rule names it.
    fn unit_of(&self, pid: u32, name: Option<&str>) -> Option<&str> {
        self.rules
            .iter()
            .find(|rule| match &rule.who {
                PeerMatch::Pid(wanted) => *wanted == pid,
                PeerMatch::Exe(wanted) => name.is_some_and(|name| name == comm_cut(wanted)),
            })
            .map(|rule| rule.unit.as_str())
    }
}

/// The part of a program name the kernel keeps in `comm` (15 bytes, on a character boundary).
fn comm_cut(name: &str) -> &str {
    let mut end = name.len().min(15);
    while !name.is_char_boundary(end) {
        end -= 1;
    }
    &name[..end]
}

fn parse_rule(entry: &str) -> Result<PeerUnitRule, PeerUnitsError> {
    let bad = || PeerUnitsError::BadEntry(entry.to_owned());
    let (who, unit) = entry.split_once('=').ok_or_else(bad)?;
    let (who, unit) = (who.trim(), unit.trim());
    if who.is_empty() || unit.is_empty() {
        return Err(bad());
    }
    let named = unit.strip_suffix(".service").is_some_and(|n| !n.is_empty());
    if !named || unit.contains('/') {
        return Err(PeerUnitsError::BadUnit(unit.to_owned()));
    }
    let who = match who.parse::<u32>() {
        Ok(pid) => PeerMatch::Pid(pid),
        Err(_) => PeerMatch::Exe(who.to_owned()),
    };
    Ok(PeerUnitRule {
        who,
        unit: unit.to_owned(),
    })
}

/// The program name of a process: `/proc/<pid>/comm`.
#[derive(Debug, Clone, Copy, Default)]
pub struct ProcComm;

impl ProcComm {
    /// The program name of process `pid`, `None` if it cannot be read.
    pub fn name_of(&self, pid: u32) -> Option<String> {
        let text = std::fs::read_to_string(format!("/proc/{pid}/comm")).ok()?;
        Some(text.trim_end_matches('\n').to_owned())
    }
}

/// The role a test unit stands for: the units of the desktop's own processes, the only ones a
/// jailed scenario needs to be.
fn role_of_unit(unit: &str) -> Option<RoleName> {
    match unit {
        "sill.service" => Some(roles::SHELL),
        "cuad.service" => Some(roles::AGENT_TIER),
        "transom.service" => Some(roles::PORTAL),
        _ => None,
    }
}

/// The standing a peer of `unit` is made to have: a service of the unit's role, proven all the
/// way (nothing real stands behind it).
fn standing_of_unit(unit: &str) -> Option<Standing> {
    Some(Standing::Service {
        app: AppKey::new("org.quire.TestOnly").ok()?,
        role: role_of_unit(unit)?,
        proof: Proof::Full,
        pin: Pinning::Reopened,
    })
}

/// A [`CallerCheck`] that answers from a [`PeerUnitTable`] first and from `inner` otherwise.
#[derive(Debug)]
pub struct DebugPeerUnits {
    table: PeerUnitTable,
    inner: Arc<dyn CallerCheck>,
    connection: Connection,
}

impl DebugPeerUnits {
    /// The wrapper over `inner`, reading program names from `/proc` and asking the bus of
    /// `connection` for a sender's process.
    pub fn new(
        table: PeerUnitTable,
        inner: Arc<dyn CallerCheck>,
        connection: Connection,
    ) -> DebugPeerUnits {
        DebugPeerUnits {
            table,
            inner,
            connection,
        }
    }

    /// The standing the table gives `sender`, if a rule matches its process.
    async fn matched(&self, sender: &str) -> Option<Standing> {
        let name = BusName::try_from(sender).ok()?;
        let pid = DBusProxy::new(&self.connection)
            .await
            .ok()?
            .get_connection_unix_process_id(name)
            .await
            .ok()?;
        let comm = ProcComm.name_of(pid);
        standing_of_unit(self.table.unit_of(pid, comm.as_deref())?)
    }
}

impl CallerCheck for DebugPeerUnits {
    fn who<'a>(&'a self, sender: &'a str) -> WhoFuture<'a> {
        Box::pin(async move {
            match self.matched(sender).await {
                Some(standing) => Ok(standing),
                None => self.inner.who(sender).await,
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_parses_names_and_pids_and_refuses_the_rest() {
        let ok = PeerUnitTable::parse(" sill = sill.service , 7=cuad.service,, ").expect("ok");
        assert_eq!(
            ok.rules(),
            [
                PeerUnitRule {
                    who: PeerMatch::Exe("sill".to_owned()),
                    unit: "sill.service".to_owned()
                },
                PeerUnitRule {
                    who: PeerMatch::Pid(7),
                    unit: "cuad.service".to_owned()
                },
            ]
        );
        assert!(PeerUnitTable::parse("").expect("empty").rules().is_empty());
        let bad = [
            ("sill", PeerUnitsError::BadEntry("sill".to_owned())),
            (
                "=a.service",
                PeerUnitsError::BadEntry("=a.service".to_owned()),
            ),
            ("sill=", PeerUnitsError::BadEntry("sill=".to_owned())),
            ("sill=sill", PeerUnitsError::BadUnit("sill".to_owned())),
            (
                "sill=.service",
                PeerUnitsError::BadUnit(".service".to_owned()),
            ),
            (
                "sill=a/b.service",
                PeerUnitsError::BadUnit("a/b.service".to_owned()),
            ),
        ];
        for (text, want) in bad {
            assert_eq!(PeerUnitTable::parse(text), Err(want), "{text}");
        }
    }

    #[test]
    fn a_rule_matches_by_pid_or_name_first_match_wins_and_the_comm_is_cut() {
        let table = PeerUnitTable::parse("2=cuad.service,sill=sill.service").expect("a table");
        assert_eq!(table.unit_of(2, Some("sill")), Some("cuad.service"));
        assert_eq!(table.unit_of(1, Some("sill")), Some("sill.service"));
        assert_eq!(table.unit_of(3, Some("other")), None);
        assert_eq!(table.unit_of(3, None), None);
        assert_eq!(comm_cut("a-very-long-program-name"), "a-very-long-pro");
    }

    #[test]
    fn a_test_unit_stands_as_the_role_of_the_desktops_own_unit_and_no_other() {
        for (unit, role) in [
            ("sill.service", roles::SHELL),
            ("cuad.service", roles::AGENT_TIER),
            ("transom.service", roles::PORTAL),
        ] {
            assert!(
                matches!(standing_of_unit(unit), Some(Standing::Service { role: got, .. }) if got == role),
                "{unit}"
            );
        }
        assert!(standing_of_unit("firefox.service").is_none());
    }
}
