//! The fake process table for hand-outs: `FakeCgroups` answers `PeerCgroups` from a map of
//! process id to the text of its `/proc/<pid>/cgroup`, so a test says which unit a bus peer
//! runs in without reading `/proc`. A clone shares the map.

use casement_ipc::IpcError;
use casement_ipc::dbus::PeerCgroups;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, PoisonError};

/// A process table standing in for `/proc`.
#[derive(Debug, Clone, Default)]
pub struct FakeCgroups {
    texts: Arc<Mutex<BTreeMap<u32, String>>>,
}

impl FakeCgroups {
    /// A table with no process.
    pub fn new() -> FakeCgroups {
        FakeCgroups::default()
    }

    /// Process `pid` runs in the user service `unit` (its cgroup file names it as systemd's
    /// would).
    pub fn run_in_service(&self, pid: u32, unit: &str) {
        let text = format!("0::/user.slice/user-1000.slice/user@1000.service/app.slice/{unit}\n");
        self.run_with_text(pid, &text);
    }

    /// Process `pid`'s cgroup file reads `text`.
    pub fn run_with_text(&self, pid: u32, text: &str) {
        self.texts
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(pid, text.to_owned());
    }
}

impl PeerCgroups for FakeCgroups {
    fn cgroup_of(&self, pid: u32) -> Result<String, IpcError> {
        self.texts
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&pid)
            .cloned()
            .ok_or_else(|| IpcError::Bus(format!("no process {pid}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_process_reads_back_its_cgroup_and_clones_share_the_table() {
        let fake = FakeCgroups::new();
        let reader = fake.clone();
        fake.run_in_service(42, "sill.service");
        let text = reader.cgroup_of(42).expect("a known process");
        assert!(text.trim_end().ends_with("/sill.service"), "{text}");
        assert!(reader.cgroup_of(43).is_err(), "an unknown process");
    }
}
