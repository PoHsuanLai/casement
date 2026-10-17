//! The `session-ready` line: the shell's bar and dock were first shown.

use super::stamp::Stamp;
use casement_protocols::vocab::ShellRole;

/// The shell surfaces whose first showing ends the session's start.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellSurface {
    /// The menu bar.
    Bar,
    /// The dock.
    Dock,
}

impl ShellSurface {
    /// The surface a shell role names, when it is one of these.
    pub fn of(role: ShellRole) -> Option<ShellSurface> {
        match role {
            ShellRole::Bar => Some(ShellSurface::Bar),
            ShellRole::Dock => Some(ShellSurface::Dock),
            _ => None,
        }
    }
}

/// The start of a session, complete: what the `session-ready` line says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionReady {
    /// When casement started (its clock's epoch).
    pub started_at: Stamp,
    /// When the shell's connection was taken in.
    pub shell_connected_at: Stamp,
    /// When the later of the shell's bar and dock was first shown.
    pub shell_ready_at: Stamp,
}

impl SessionReady {
    /// The line, message first, then `key=value` fields.
    pub fn line(&self) -> String {
        format!(
            "session-ready started_at={} shell_connected_at={} shell_ready_at={}",
            self.started_at, self.shell_connected_at, self.shell_ready_at
        )
    }
}

/// Where the session's start is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionStage {
    /// Waiting for the shell.
    Waiting {
        /// When casement started.
        started_at: Stamp,
        /// When the shell connected, once it has.
        shell_connected_at: Option<Stamp>,
        /// When the bar was first shown, once it has been.
        bar_at: Option<Stamp>,
        /// When the dock was first shown, once it has been.
        dock_at: Option<Stamp>,
    },
    /// Reported.
    Done,
}

impl SessionStage {
    /// A session that started at `started_at`.
    pub fn starting(started_at: Stamp) -> SessionStage {
        SessionStage::Waiting {
            started_at,
            shell_connected_at: None,
            bar_at: None,
            dock_at: None,
        }
    }

    /// Whether frames are worth looking at for the shell's surfaces: the shell connected and the
    /// start is not over.
    pub fn watches_surfaces(&self) -> bool {
        matches!(
            self,
            SessionStage::Waiting {
                shell_connected_at: Some(_),
                ..
            }
        )
    }

    /// The shell's connection was taken in at `at`. Only the first counts.
    #[must_use]
    pub fn shell_connected(self, at: Stamp) -> SessionStage {
        match self {
            SessionStage::Waiting {
                started_at,
                shell_connected_at,
                bar_at,
                dock_at,
            } => SessionStage::Waiting {
                started_at,
                shell_connected_at: shell_connected_at.or(Some(at)),
                bar_at,
                dock_at,
            },
            SessionStage::Done => SessionStage::Done,
        }
    }

    /// A frame at `at` showed the shell's `surface`: the record, once both are up. Only the first
    /// showing of each counts.
    #[must_use]
    pub fn shown(self, surface: ShellSurface, at: Stamp) -> (SessionStage, Option<SessionReady>) {
        let SessionStage::Waiting {
            started_at,
            shell_connected_at,
            bar_at,
            dock_at,
        } = self
        else {
            return (SessionStage::Done, None);
        };
        let (bar_at, dock_at) = match surface {
            ShellSurface::Bar => (bar_at.or(Some(at)), dock_at),
            ShellSurface::Dock => (bar_at, dock_at.or(Some(at))),
        };
        match (bar_at, dock_at) {
            (Some(bar), Some(dock)) => {
                let connected = shell_connected_at.unwrap_or(started_at);
                let ready = SessionReady {
                    started_at,
                    shell_connected_at: connected,
                    shell_ready_at: bar.or_later(dock).or_later(connected),
                };
                (SessionStage::Done, Some(ready))
            }
            _ => (
                SessionStage::Waiting {
                    started_at,
                    shell_connected_at,
                    bar_at,
                    dock_at,
                },
                None,
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_session_is_ready_when_the_bar_and_the_dock_have_both_been_shown() {
        let stage = SessionStage::starting(Stamp(1_000));
        assert!(!stage.watches_surfaces(), "no shell yet");
        let stage = stage.shell_connected(Stamp(2_000));
        assert!(stage.watches_surfaces());
        // The second connection and a repeated bar do not move anything.
        let stage = stage.shell_connected(Stamp(2_500));
        let (stage, none) = stage.shown(ShellSurface::Bar, Stamp(3_000));
        assert_eq!(none, None);
        let (stage, none) = stage.shown(ShellSurface::Bar, Stamp(3_500));
        assert_eq!(none, None);
        let (stage, ready) = stage.shown(ShellSurface::Dock, Stamp(4_000));
        assert_eq!(
            ready.map(|ready| ready.line()),
            Some(
                "session-ready started_at=1000 shell_connected_at=2000 shell_ready_at=4000"
                    .to_owned()
            )
        );
        assert_eq!(stage, SessionStage::Done);
        assert!(!stage.watches_surfaces());
        assert_eq!(
            stage.shown(ShellSurface::Dock, Stamp(5_000)),
            (SessionStage::Done, None)
        );
    }

    #[test]
    fn only_the_bar_and_the_dock_end_the_start() {
        // name, role, surface
        let cases = [
            ("bar", ShellRole::Bar, Some(ShellSurface::Bar)),
            ("dock", ShellRole::Dock, Some(ShellSurface::Dock)),
            ("launcher", ShellRole::Launcher, None),
            ("wallpaper", ShellRole::Wallpaper, None),
        ];
        for (name, role, want) in cases {
            assert_eq!(ShellSurface::of(role), want, "{name}");
        }
    }
}
