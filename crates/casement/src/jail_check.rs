//! The structural jail check (§6.1), compiled only under `testing` or `debug-inject`: a binary
//! that can mint Physical, Virtual or Test input refuses to start unless it is inside
//! `~/desktop/harness/jail.sh`. It only stats paths and reads `/proc/1/comm`; it never opens a
//! device.

use crate::cli::StartConfig;
use casement_backend::BackendChoice;
use casement_privacy::{JailBreach, breach};
use std::path::Path;

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

/// Whether a start with `config` may go ahead under `root` (`/` in the binary). The jail's
/// conditions are `casement_privacy::breach`'s, shared with `casementctl`.
pub fn require_start(config: &StartConfig, root: &Path) -> Result<(), JailRefusal> {
    if config.backend != BackendChoice::Headless {
        return Err(JailRefusal::Backend);
    }
    match breach(root, rustix::process::getuid().as_raw()) {
        None => Ok(()),
        Some(breach) => Err(JailRefusal::Outside(breach)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{self, Command};
    use std::fs;

    fn start(backend: &str) -> StartConfig {
        let args = ["--backend", backend].map(str::to_owned);
        let Ok(Command::Run(start)) = cli::parse_args(&args) else {
            panic!("the arguments parse to a run");
        };
        *start
    }

    fn jail_tree(pid_one: &str) -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("temp dir");
        fs::create_dir_all(root.path().join("proc/1")).expect("proc");
        fs::write(root.path().join("proc/1/comm"), pid_one).expect("comm");
        root
    }

    #[test]
    fn a_headless_start_inside_the_jail_goes_ahead() {
        assert_eq!(
            require_start(&start("headless"), jail_tree("bwrap\n").path()),
            Ok(())
        );
    }

    #[test]
    fn a_start_outside_the_jail_or_on_a_real_backend_is_refused() {
        assert_eq!(
            require_start(&start("headless"), jail_tree("systemd\n").path()),
            Err(JailRefusal::Outside(JailBreach::NotPidNamespace))
        );
        assert_eq!(
            require_start(&start("tty"), jail_tree("bwrap\n").path()),
            Err(JailRefusal::Backend)
        );
    }
}
