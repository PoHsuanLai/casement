//! The structural jail check (20-FREEZE-PLAN §6.1): four conditions read from paths and never
//! from the environment. The compositor's test builds and `casementctl` both refuse to run
//! outside the jail, and neither opens a device: this only stats paths and reads `/proc/1/comm`.
//! (It lives here because `casementctl` links this crate and not the compositor.)

use std::path::Path;

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

impl std::fmt::Display for JailBreach {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            JailBreach::InputDevices => "/dev/input exists",
            JailBreach::SessionSocket => "the user's wayland-0 socket exists",
            JailBreach::SystemBus => "the system bus socket exists",
            JailBreach::NotPidNamespace => "PID 1 is not bwrap",
        })
    }
}

/// The first condition under `root` (`/` in the binary, a scratch tree in tests) that does not
/// hold for the user `uid`, or `None` inside the jail.
pub fn breach(root: &Path, uid: u32) -> Option<JailBreach> {
    let present = |path: String| root.join(path).symlink_metadata().is_ok();
    let pid_one = std::fs::read_to_string(root.join("proc/1/comm")).unwrap_or_default();
    if present("dev/input".to_owned()) {
        Some(JailBreach::InputDevices)
    } else if present(format!("run/user/{uid}/wayland-0")) {
        Some(JailBreach::SessionSocket)
    } else if present("run/dbus/system_bus_socket".to_owned()) {
        Some(JailBreach::SystemBus)
    } else if pid_one.trim() != "bwrap" {
        Some(JailBreach::NotPidNamespace)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree(pid_one: &str) -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("temp dir");
        std::fs::create_dir_all(root.path().join("proc/1")).expect("proc");
        std::fs::write(root.path().join("proc/1/comm"), pid_one).expect("comm");
        root
    }

    #[test]
    fn an_empty_namespace_under_bwrap_holds() {
        assert_eq!(breach(tree("bwrap\n").path(), 1000), None);
    }

    #[test]
    fn each_leak_is_named_in_the_order_the_plan_lists_them() {
        let tree = tree("bwrap\n");
        let at = |path: &str| {
            let full = tree.path().join(path);
            std::fs::create_dir_all(full.parent().expect("parent")).expect("dirs");
            std::fs::write(full, "").expect("write");
        };
        at("run/dbus/system_bus_socket");
        assert_eq!(breach(tree.path(), 1000), Some(JailBreach::SystemBus));
        at("run/user/1000/wayland-0");
        assert_eq!(breach(tree.path(), 1000), Some(JailBreach::SessionSocket));
        at("dev/input");
        assert_eq!(breach(tree.path(), 1000), Some(JailBreach::InputDevices));
    }

    #[test]
    fn a_host_pid_one_is_not_the_jail() {
        assert_eq!(
            breach(tree("systemd\n").path(), 1000),
            Some(JailBreach::NotPidNamespace)
        );
    }
}
