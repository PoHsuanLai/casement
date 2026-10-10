//! Test unit overrides (feature `debug-inject` only): a table that says "the peer running the
//! program `sill` is unit `sill.service`", for a jailed desktop scenario that has no systemd and
//! so no `/proc/<pid>/cgroup` naming a unit. [`DebugPeerUnits`] is a [`PeerCgroups`] that answers
//! from the table first and falls back to the real seam. This file is compiled only under
//! `debug-inject`; a release build has no such type, and nothing in it reads the environment
//! (the binary parses its variable and hands the text to [`PeerUnitTable::parse`]).

use super::compositor1::PeerCgroups;
use crate::IpcError;
use std::sync::Arc;

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

/// The program name of a process: `/proc/<pid>/comm`. A seam, with the real one [`ProcComm`].
pub trait PeerNames: std::fmt::Debug + Send + Sync + 'static {
    /// The program name of process `pid`, `None` if it cannot be read.
    fn name_of(&self, pid: u32) -> Option<String>;
}

/// The real seam: `/proc/<pid>/comm`.
#[derive(Debug, Clone, Copy, Default)]
pub struct ProcComm;

impl PeerNames for ProcComm {
    fn name_of(&self, pid: u32) -> Option<String> {
        let text = std::fs::read_to_string(format!("/proc/{pid}/comm")).ok()?;
        Some(text.trim_end_matches('\n').to_owned())
    }
}

/// A [`PeerCgroups`] that answers from a [`PeerUnitTable`] first and from `inner` otherwise.
#[derive(Debug)]
pub struct DebugPeerUnits {
    table: PeerUnitTable,
    names: Arc<dyn PeerNames>,
    inner: Arc<dyn PeerCgroups>,
}

impl DebugPeerUnits {
    /// The wrapper over `inner`, reading program names through `names`.
    pub fn new(
        table: PeerUnitTable,
        names: Arc<dyn PeerNames>,
        inner: Arc<dyn PeerCgroups>,
    ) -> DebugPeerUnits {
        DebugPeerUnits {
            table,
            names,
            inner,
        }
    }
}

impl PeerCgroups for DebugPeerUnits {
    fn cgroup_of(&self, pid: u32) -> Result<String, IpcError> {
        let name = self.names.name_of(pid);
        match self.table.unit_of(pid, name.as_deref()) {
            Some(unit) => Ok(format!("0::/user.slice/app.slice/{unit}\n")),
            None => self.inner.cgroup_of(pid),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[derive(Debug)]
    struct Names(BTreeMap<u32, &'static str>);

    impl PeerNames for Names {
        fn name_of(&self, pid: u32) -> Option<String> {
            self.0.get(&pid).map(|name| (*name).to_owned())
        }
    }

    #[derive(Debug)]
    struct Fixed;

    impl PeerCgroups for Fixed {
        fn cgroup_of(&self, pid: u32) -> Result<String, IpcError> {
            match pid {
                1 => Ok("0::/user.slice/app.slice/cuad.service\n".to_owned()),
                _ => Err(IpcError::Bus(format!("no process {pid}"))),
            }
        }
    }

    fn wrapper(table: &str) -> DebugPeerUnits {
        let names = Names(BTreeMap::from([(1, "sill"), (2, "sill"), (3, "other")]));
        DebugPeerUnits::new(
            PeerUnitTable::parse(table).expect("a table"),
            Arc::new(names),
            Arc::new(Fixed),
        )
    }

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
    fn the_table_wins_over_the_real_seam_and_the_real_seam_answers_the_rest() {
        let units = wrapper("sill=sill.service");
        let by_name = units.cgroup_of(2).expect("matched by name");
        assert!(by_name.trim_end().ends_with("/sill.service"), "{by_name}");
        // Pid 1 is also named sill: the table beats the inner seam's cuad.service.
        let beaten = units.cgroup_of(1).expect("matched by name");
        assert!(beaten.trim_end().ends_with("/sill.service"), "{beaten}");
        // An unmatched peer is the inner seam's answer, error included.
        assert!(units.cgroup_of(3).is_err());
        let plain = wrapper("");
        assert!(plain.cgroup_of(1).expect("inner").contains("cuad.service"));
    }

    #[test]
    fn a_pid_rule_and_the_first_match_and_the_comm_cut_work() {
        let units = wrapper("2=cuad.service,sill=sill.service");
        assert!(
            units
                .cgroup_of(2)
                .expect("pid")
                .ends_with("/cuad.service\n")
        );
        assert!(
            units
                .cgroup_of(1)
                .expect("name")
                .ends_with("/sill.service\n")
        );
        assert_eq!(comm_cut("a-very-long-program-name"), "a-very-long-pro");
    }
}
