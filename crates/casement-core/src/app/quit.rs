//! Quitting, force quitting and not responding, the macOS way: the pure parts. How long a
//! client has to answer a ping, when to ping, which apps the Force Quit window lists, and what
//! a force quit asks the edge to stop.

use super::{App, AppKey, AppLife};
use crate::effect::Held;
use crate::ids::ClientKey;
use std::collections::BTreeMap;
use std::time::Duration;

/// How long a client has to answer a ping before its app is marked not responding.
pub const PING_DEADLINE: Duration = Duration::from_secs(4);

/// The least time between two pings to one client: input arrives many times a second, and a
/// ping that was answered a moment ago proves enough.
pub const PING_GAP: Duration = Duration::from_secs(2);

/// How long an app asked to quit shows as quitting before it is taken for running again (it
/// kept its windows: a "Save changes?" sheet, or it declined).
pub const QUIT_GRACE: Duration = Duration::from_secs(3);

/// How long an app asked to stop (SIGTERM) has before its processes are killed (SIGKILL).
pub const KILL_GRACE: Duration = Duration::from_secs(3);

/// Whether to ping a client now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pinging {
    /// Send a ping and time the answer.
    Send,
    /// Do nothing.
    Hold,
}

/// Whether a ping to a client is due, from its app's life (none when the client serves no app
/// yet), whether a ping is out, and whether one went a moment ago.
///
/// ```
/// use casement_core::app::{AppLife, Pinging, ping_plan};
/// use casement_core::Held;
///
/// assert_eq!(ping_plan(Some(&AppLife::Running), Held::No, Held::No), Pinging::Send);
/// assert_eq!(ping_plan(Some(&AppLife::Running), Held::Yes, Held::No), Pinging::Hold);
/// ```
pub fn ping_plan(life: Option<&AppLife>, outstanding: Held, recent: Held) -> Pinging {
    match (life, outstanding, recent) {
        (None | Some(AppLife::NotResponding { .. }), _, _) => Pinging::Hold,
        (Some(_), Held::Yes, _) | (Some(_), _, Held::Yes) => Pinging::Hold,
        (Some(_), Held::No, Held::No) => Pinging::Send,
    }
}

/// Whether an app answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Answer {
    /// It answers pings.
    Responding,
    /// It stopped answering.
    NotResponding,
}

/// One row of the Force Quit Applications window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForceQuitRow {
    /// The app.
    pub app: AppKey,
    /// Whether it answers.
    pub answer: Answer,
}

/// The rows of the Force Quit window: every app that is running, in key order.
pub fn force_quit_rows(apps: &BTreeMap<AppKey, App>) -> Vec<ForceQuitRow> {
    apps.values()
        .filter(|app| !matches!(app.life, AppLife::Launching { .. }))
        .map(|app| ForceQuitRow {
            app: app.key.clone(),
            answer: match app.life {
                AppLife::NotResponding { .. } => Answer::NotResponding,
                _ => Answer::Responding,
            },
        })
        .collect()
}

/// The signal that stops an app's processes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopSignal {
    /// `SIGTERM`: stop, and let the process clean up.
    Term,
    /// `SIGKILL`: stop now.
    Kill,
}

/// An app's processes are to be stopped. `clients` are the connections that serve this app
/// and no other; the edge reads their processes from their socket credentials, never by name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Termination {
    /// The app.
    pub app: AppKey,
    /// The connections whose processes are stopped.
    pub clients: Vec<ClientKey>,
    /// The signal.
    pub signal: StopSignal,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::Mono;

    #[test]
    fn a_ping_is_due_only_for_an_app_that_answers_and_has_none_out() {
        let stuck = AppLife::NotResponding {
            since: Mono(Duration::ZERO),
        };
        let cases = [
            ("no app", None, Held::No, Held::No, Pinging::Hold),
            (
                "running",
                Some(AppLife::Running),
                Held::No,
                Held::No,
                Pinging::Send,
            ),
            (
                "windowless",
                Some(AppLife::Windowless),
                Held::No,
                Held::No,
                Pinging::Send,
            ),
            (
                "one out",
                Some(AppLife::Running),
                Held::Yes,
                Held::No,
                Pinging::Hold,
            ),
            (
                "one a moment ago",
                Some(AppLife::Running),
                Held::No,
                Held::Yes,
                Pinging::Hold,
            ),
            (
                "already stuck",
                Some(stuck),
                Held::No,
                Held::No,
                Pinging::Hold,
            ),
        ];
        for (name, life, out, recent, want) in cases {
            assert_eq!(ping_plan(life.as_ref(), out, recent), want, "{name}");
        }
    }
}
