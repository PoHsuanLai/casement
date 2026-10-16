//! Test builds only (`testing`, `debug-inject`): the desktop's list of its own programs gains the
//! rows of `desktop-callers.toml` in the injected folder (`--system-dir`), owned by the person
//! running the test. The desktop harness runs casement in a jail with no `/etc/quire`, so without
//! it nobody is the shell, Settings or the portal and none of them can connect; the harness
//! names its build-folder programs here instead. The rows are the library's developer rows
//! (`Proof::Dev`, below every real proof): the system table still wins where both name a
//! program, and `dbus::require` and `client_standing` let exactly that proof in. A release build
//! compiles this file out (and the library's `dev-rows` feature with it).
//!
//! The file is read only when it is the person's own: a plain file (a link is refused), owned
//! by the person or by root, writable by nobody else. One that is not adds no row and is logged
//! once; one that is not there is the usual case and says nothing.

use crate::privacy::SystemDir;
use casement_ipc::dbus::peer::{Table, Uid};
use casement_ipc::dbus::{LoadError, RealProc, TableFiles};
use std::io;
use std::path::Path;

/// The file's name in the injected folder.
pub(super) const FILE: &str = "desktop-callers.toml";

/// `shipped` with the test caller rows of `system` after it, when `system` is an injected
/// folder; the table as it is otherwise.
pub(super) fn merged(shipped: Table, system: &SystemDir) -> Table {
    match system {
        SystemDir::Etc => shipped,
        SystemDir::At(dir) => shipped.merge(rows_in(&RealProc::system(), &dir.join(FILE))),
    }
}

/// The user running this process.
fn person() -> Uid {
    Uid::new(rustix::process::getuid().as_raw())
}

/// The test rows in `path`; none when it is missing, and none (with one log line) when it cannot
/// be used.
fn rows_in(fs: &RealProc, path: &Path) -> Table {
    match TableFiles::load_dev(fs, path, person()) {
        Ok(rows) => rows,
        Err(LoadError::Unreadable { source, .. }) if source.kind() == io::ErrorKind::NotFound => {
            Table::empty()
        }
        Err(error) => {
            tracing::warn!(
                %error,
                path = %path.display(),
                "TEST ONLY: the test list of the desktop's programs could not be used, so it adds none"
            );
            Table::empty()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clients::{Judge, Standing};
    use casement_core::privilege::{ClientStanding, Floor, ServiceRole};
    use porter_core::PeerIdentity;
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::net::UnixStream;
    use std::path::PathBuf;

    /// A scratch injected folder holding `desktop-callers.toml` with `text`, mode `mode`.
    fn folder_with(text: &str, mode: u32) -> (tempfile::TempDir, SystemDir) {
        let dir = tempfile::tempdir().expect("a scratch folder");
        let file = dir.path().join(FILE);
        std::fs::write(&file, text).expect("the rows file");
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(mode)).expect("its mode");
        let at = SystemDir::At(dir.path().to_path_buf());
        (dir, at)
    }

    /// Rows that make `exe` the shell.
    fn shell_rows(exe: &Path) -> String {
        format!(
            "[[peer]]\napp = \"org.quire.Shell\"\nrole = \"shell\"\nexe = [\"{}\"]\n",
            exe.display()
        )
    }

    fn this_binary() -> PathBuf {
        std::env::current_exe().expect("this binary")
    }

    /// What a judge over `table` makes of this test binary connecting.
    fn arrival_under(table: Table, system: SystemDir) -> (ClientStanding, PeerIdentity) {
        let (ours, _theirs) = UnixStream::pair().expect("a socket pair");
        let arrival = Judge::new(table, system).arrival(&ours, &Floor::empty());
        let Standing::Judged { standing, .. } = arrival.standing else {
            panic!("a public client is always judged");
        };
        (standing, arrival.peer)
    }

    fn shell() -> ServiceRole {
        ServiceRole::parse("shell").expect("a role")
    }

    #[test]
    fn a_row_in_the_injected_folder_makes_a_program_a_service_of_its_role() {
        let (_dir, system) = folder_with(&shell_rows(&this_binary()), 0o600);
        let table = merged(Table::empty(), &system);
        assert_eq!(table.rows().len(), 1);
        let (standing, peer) = arrival_under(table, system);
        assert!(
            matches!(&standing, ClientStanding::Service { role, .. } if *role == shell()),
            "{standing:?}"
        );
        assert!(matches!(peer, PeerIdentity::Proven(_)), "{peer:?}");
    }

    #[test]
    fn without_the_file_nothing_changes() {
        let dir = tempfile::tempdir().expect("a scratch folder");
        let system = SystemDir::At(dir.path().to_path_buf());
        assert!(merged(Table::empty(), &system).rows().is_empty());
        let (standing, peer) = arrival_under(merged(Table::empty(), &system), system);
        assert!(
            !matches!(standing, ClientStanding::Service { .. }),
            "{standing:?}"
        );
        assert!(matches!(peer, PeerIdentity::Unproven));
    }

    #[test]
    fn a_file_others_can_write_or_that_is_a_link_adds_nothing() {
        for mode in [0o666, 0o662, 0o620] {
            let (_dir, system) = folder_with(&shell_rows(&this_binary()), mode);
            assert!(
                merged(Table::empty(), &system).rows().is_empty(),
                "{mode:o}"
            );
        }
        let (dir, _) = folder_with(&shell_rows(&this_binary()), 0o600);
        let real = dir.path().join(FILE);
        let elsewhere = dir.path().join("real.toml");
        std::fs::rename(&real, &elsewhere).expect("moved");
        std::os::unix::fs::symlink(&elsewhere, &real).expect("a link");
        let linked = SystemDir::At(dir.path().to_path_buf());
        assert!(merged(Table::empty(), &linked).rows().is_empty());
    }

    #[test]
    fn a_file_that_is_not_a_table_adds_nothing() {
        let (_dir, system) = folder_with("this is [not a table", 0o600);
        assert!(merged(Table::empty(), &system).rows().is_empty());
    }

    #[test]
    fn the_system_floor_folder_adds_no_test_rows() {
        let table = merged(Table::empty(), &SystemDir::Etc);
        assert!(table.rows().is_empty());
    }
}
