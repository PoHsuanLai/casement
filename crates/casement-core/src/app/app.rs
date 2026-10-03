//! An app's record: its processes, windows, life and visibility.

use super::AppKey;
use crate::ids::{Pid, WindowId};
use crate::time::Mono;
use casement_protocols::vocab::{AppLifeKind, Attention};
use std::collections::BTreeSet;

/// The processes an app's clients run in.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PidSet(pub BTreeSet<Pid>);

/// One app the model knows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct App {
    /// Who it is.
    pub key: AppKey,
    /// Its processes.
    pub pids: PidSet,
    /// Its windows, in its own order (the order activation raises them in, 03 A4).
    pub windows: Vec<WindowId>,
    /// Where it is in its life.
    pub life: AppLife,
    /// Whether it is hidden (Cmd+H).
    pub visibility: AppVisibility,
    /// Whether it asks for attention (the dock bounce).
    pub attention: Attention,
}

/// Where an app is in its life.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppLife {
    /// Launched, no window yet.
    Launching {
        /// When the launch began.
        since: Mono,
    },
    /// Running with windows.
    Running,
    /// Running with no window (it still answers its menu, 03 A1).
    Windowless,
    /// Asked to quit.
    Quitting(QuitPhase),
    /// It stopped answering pings.
    NotResponding {
        /// When the first ping went unanswered.
        since: Mono,
    },
}

/// How far a quit has gone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuitPhase {
    /// The app was asked to close its windows.
    Asked {
        /// When it was asked.
        since: Mono,
    },
    /// The grace period passed; it is being stopped.
    Stopping,
}

/// Whether an app's windows are hidden as a group (app-level, unlike minimising).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppVisibility {
    /// Its windows are shown.
    Shown,
    /// Hidden; unhiding restores these windows in this order (minimised ones stay minimised).
    Hidden {
        /// The stacking order to restore.
        restore_order: Vec<WindowId>,
    },
}

impl AppLife {
    /// The life as the wire names it.
    pub fn kind(&self) -> AppLifeKind {
        match self {
            AppLife::Launching { .. } => AppLifeKind::Launching,
            AppLife::Running => AppLifeKind::Running,
            AppLife::Windowless => AppLifeKind::Windowless,
            AppLife::Quitting(_) => AppLifeKind::Quitting,
            AppLife::NotResponding { .. } => AppLifeKind::NotResponding,
        }
    }
}
