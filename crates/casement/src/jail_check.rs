//! The structural jail check (§6.1), compiled only under `testing` or `debug-inject`: a binary
//! that can mint Physical, Virtual or Test input refuses to start unless it is inside
//! `~/desktop/harness/jail.sh`. It only stats paths and reads `/proc/1/comm`; it never opens a
//! device.

use crate::cli::StartConfig;
use casement_backend::BackendChoice;
use std::path::Path;

/// Whether this process is in the jail.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JailCheck {
    /// Every condition holds.
    Inside,
    /// This condition (the first found) does not.
    Outside(JailBreach),
}

/// A condition of the jail that does not hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JailBreach {
    /// `/dev/input` exists.
    InputDevices,
    /// `/run/user/$UID/wayland-0` (the user's session) exists.
    SessionSocket,
    /// `/run/dbus/system_bus_socket` exists.
    SystemBus,
    /// PID 1 of this namespace is not bwrap.
    NotPidNamespace,
}

/// Checks the jail's conditions under `root` (`/` in the binary, a scratch tree in tests).
pub fn require_jail(root: &Path) -> JailCheck {
    let uid = rustix::process::getuid().as_raw();
    let present = |path: &str| root.join(path).symlink_metadata().is_ok();
    let pid_one = std::fs::read_to_string(root.join("proc/1/comm")).unwrap_or_default();
    if present("dev/input") {
        JailCheck::Outside(JailBreach::InputDevices)
    } else if present(&format!("run/user/{uid}/wayland-0")) {
        JailCheck::Outside(JailBreach::SessionSocket)
    } else if present("run/dbus/system_bus_socket") {
        JailCheck::Outside(JailBreach::SystemBus)
    } else if pid_one.trim_end() != "bwrap" {
        JailCheck::Outside(JailBreach::NotPidNamespace)
    } else {
        JailCheck::Inside
    }
}

/// Why a binary that can mint test input will not start.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
#[non_exhaustive]
pub enum JailRefusal {
    /// The build refuses every backend but headless.
    #[error("this build refuses every backend but headless")]
    Backend,
    /// The process is outside the jail.
    #[error("this build runs only inside the jail ({0:?})")]
    Outside(JailBreach),
}

/// Whether a start with `config` may go ahead under `root` (`/` in the binary).
pub fn require_start(config: &StartConfig, root: &Path) -> Result<(), JailRefusal> {
    if config.backend != BackendChoice::Headless {
        return Err(JailRefusal::Backend);
    }
    match require_jail(root) {
        JailCheck::Inside => Ok(()),
        JailCheck::Outside(breach) => Err(JailRefusal::Outside(breach)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn jail_tree() -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("temp dir");
        fs::create_dir_all(root.path().join("proc/1")).expect("proc");
        fs::write(root.path().join("proc/1/comm"), "bwrap\n").expect("comm");
        fs::create_dir_all(root.path().join("dev")).expect("dev");
        root
    }

    #[test]
    fn a_jail_shaped_tree_is_inside() {
        assert_eq!(require_jail(jail_tree().path()), JailCheck::Inside);
    }

    #[test]
    fn each_breach_is_named() {
        let uid = rustix::process::getuid().as_raw();
        let input = jail_tree();
        fs::create_dir_all(input.path().join("dev/input")).expect("input");
        assert_eq!(
            require_jail(input.path()),
            JailCheck::Outside(JailBreach::InputDevices)
        );
        let session = jail_tree();
        fs::create_dir_all(session.path().join(format!("run/user/{uid}"))).expect("run");
        fs::write(session.path().join(format!("run/user/{uid}/wayland-0")), "").expect("socket");
        assert_eq!(
            require_jail(session.path()),
            JailCheck::Outside(JailBreach::SessionSocket)
        );
        let bus = jail_tree();
        fs::create_dir_all(bus.path().join("run/dbus")).expect("dbus");
        fs::write(bus.path().join("run/dbus/system_bus_socket"), "").expect("bus");
        assert_eq!(
            require_jail(bus.path()),
            JailCheck::Outside(JailBreach::SystemBus)
        );
        let host = jail_tree();
        fs::write(host.path().join("proc/1/comm"), "systemd\n").expect("comm");
        assert_eq!(
            require_jail(host.path()),
            JailCheck::Outside(JailBreach::NotPidNamespace)
        );
    }
}
