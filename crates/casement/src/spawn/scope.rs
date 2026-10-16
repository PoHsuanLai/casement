//! A program in a scope of its own: `systemd-run --user --scope`, as sill launches apps.
//!
//! The compositor's service ends every process in its control group when it stops, which is
//! right for its windows and wrong for a terminal's tmux server or a long build. A transient
//! scope puts the program under the person's service manager instead, so a restart of the
//! compositor leaves it alone. `systemd-run --scope` runs the program in place (it becomes the
//! program), so the pid the compositor holds is the program's.

use std::path::{Path, PathBuf};

/// Where the person's service manager can be asked for a scope.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Scopes {
    /// Never: a plain child (a run with no user session, a test).
    #[default]
    Off,
    /// When the manager listens under this runtime folder (`$XDG_RUNTIME_DIR`).
    Under(PathBuf),
}

impl Scopes {
    /// Scopes under `runtime`, or none when there is no runtime folder.
    pub fn under(runtime: Option<PathBuf>) -> Scopes {
        runtime.map_or(Scopes::Off, Scopes::Under)
    }

    /// Whether a manager answers now: its private socket is there. Without it `systemd-run`
    /// fails and the program it wraps is never started.
    pub(super) fn reachable(&self) -> Reach {
        match self {
            Scopes::Under(runtime) if runtime.join("systemd/private").exists() => Reach::Yes,
            Scopes::Under(_) | Scopes::Off => Reach::No,
        }
    }
}

/// Whether a manager answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Reach {
    Yes,
    No,
}

/// The longest part of a program's name kept in its unit name.
const STEM_LIMIT: usize = 32;

/// The unit name of the scope for `program` (as named, path or not): `app-casement-<name>-<pid>-
/// <nonce>.scope`. The compositor's `pid` keeps a new run's names apart from the scopes of an
/// earlier run that still holds programs; the `nonce` counts this run's starts. Anything in the
/// name outside ASCII letters and digits is `_`.
///
/// ```
/// use casement::spawn::unit_for;
///
/// assert_eq!(unit_for("/usr/bin/foot", 41, 3), "app-casement-foot-41-3.scope");
/// assert_eq!(unit_for("my term!", 1, 1), "app-casement-my_term_-1-1.scope");
/// ```
pub fn unit_for(program: &str, pid: u32, nonce: u32) -> String {
    let base = Path::new(program)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("program");
    let stem: String = base
        .chars()
        .take(STEM_LIMIT)
        .map(|c| match c.is_ascii_alphanumeric() {
            true => c,
            false => '_',
        })
        .collect();
    format!("app-casement-{stem}-{pid}-{nonce}.scope")
}

/// `argv` run in the scope `unit`.
///
/// ```
/// use casement::spawn::scope_argv;
///
/// let got = scope_argv(&["foot".to_owned()], "app-casement-foot-1-1.scope");
/// assert_eq!(got[0], "systemd-run");
/// assert_eq!(got.last().map(String::as_str), Some("foot"));
/// ```
pub fn scope_argv(argv: &[String], unit: &str) -> Vec<String> {
    ["systemd-run", "--user", "--scope", "--collect", "--quiet"]
        .iter()
        .map(|part| (*part).to_owned())
        .chain([format!("--unit={unit}"), "--".to_owned()])
        .chain(argv.iter().cloned())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scopes_need_a_manager_socket() {
        let dir = tempfile::tempdir().expect("a scratch folder");
        assert_eq!(Scopes::Off.reachable(), Reach::No);
        assert_eq!(Scopes::under(None).reachable(), Reach::No);
        let under = Scopes::under(Some(dir.path().to_owned()));
        assert_eq!(under.reachable(), Reach::No);
        std::fs::create_dir(dir.path().join("systemd")).expect("folder");
        std::fs::write(dir.path().join("systemd/private"), b"").expect("socket stand-in");
        assert_eq!(under.reachable(), Reach::Yes);
    }

    #[test]
    fn a_unit_name_has_only_letters_digits_and_dashes_of_its_own() {
        // program, want
        let cases = [
            ("foot", "app-casement-foot-7-2.scope"),
            ("/usr/bin/kitty", "app-casement-kitty-7-2.scope"),
            ("a.b c", "app-casement-a_b_c-7-2.scope"),
            ("", "app-casement-program-7-2.scope"),
        ];
        for (program, want) in cases {
            assert_eq!(unit_for(program, 7, 2), want, "{program:?}");
        }
        let long = "x".repeat(100);
        assert_eq!(
            unit_for(&long, 7, 2).len(),
            "app-casement--7-2.scope".len() + STEM_LIMIT
        );
    }
}
