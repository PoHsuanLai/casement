//! The structural jail check (20-FREEZE-PLAN §6.1): the same four conditions as
//! `casement::jail_check` and the shell `require_jail`, read from paths and never from the
//! environment. casementctl never opens a device; it only stats paths and reads `/proc/1/comm`.

use std::os::unix::fs::MetadataExt;
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
/// hold, or `None` inside the jail.
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

/// [`breach`] for this process on the real root.
pub fn breach_here() -> Option<JailBreach> {
    let uid = std::fs::metadata("/proc/self").map_or(0, |m| m.uid());
    breach(Path::new("/"), uid)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A scratch tree under the temp dir, removed when dropped.
    struct Tree(std::path::PathBuf);

    impl Tree {
        fn new(name: &str, pid_one: &str) -> Tree {
            let path = std::env::temp_dir()
                .join(format!("casementctl-jail-{}-{name}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(path.join("proc/1")).expect("proc");
            std::fs::write(path.join("proc/1/comm"), pid_one).expect("comm");
            Tree(path)
        }
    }

    impl Drop for Tree {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn an_empty_namespace_under_bwrap_holds() {
        assert_eq!(breach(&Tree::new("held", "bwrap\n").0, 1000), None);
    }

    #[test]
    fn each_leak_is_named_in_the_order_the_plan_lists_them() {
        let tree = Tree::new("leaks", "bwrap\n");
        let at = |path: &str| {
            let full = tree.0.join(path);
            std::fs::create_dir_all(full.parent().expect("parent")).expect("dirs");
            std::fs::write(full, "").expect("write");
        };
        at("run/dbus/system_bus_socket");
        assert_eq!(breach(&tree.0, 1000), Some(JailBreach::SystemBus));
        at("run/user/1000/wayland-0");
        assert_eq!(breach(&tree.0, 1000), Some(JailBreach::SessionSocket));
        at("dev/input");
        assert_eq!(breach(&tree.0, 1000), Some(JailBreach::InputDevices));
    }

    #[test]
    fn a_host_pid_one_is_not_the_jail() {
        assert_eq!(
            breach(&Tree::new("host", "systemd\n").0, 1000),
            Some(JailBreach::NotPidNamespace)
        );
    }
}
