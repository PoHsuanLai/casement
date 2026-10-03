//! What an output shows.

use casement_core::app::AppKey;
use porter_core::SpaceId;

/// What an output shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SceneMode {
    /// A Space's desktop.
    Desktop {
        /// The Space shown.
        shown: SpaceId,
    },
    /// Mission Control or App Exposé over a Space.
    Overview {
        /// The Space the overview opened on.
        shown: SpaceId,
        /// Which overview.
        kind: OverviewKind,
    },
    /// Show Desktop: windows pushed aside.
    ShowDesktop,
}

/// Which overview.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OverviewKind {
    /// Every window of the shown Space, spread out, with the Space row.
    MissionControl,
    /// One app's windows, in stacking order (`StackIndex`, then window id), never list
    /// order.
    AppExpose(AppKey),
}
