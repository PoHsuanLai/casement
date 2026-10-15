//! The requests of `quire_desktop_manager_v1`, as the model takes them. A shell makes them for
//! the person at the keyboard; an agent connection is refused them at the wire
//! (`not_permitted`), so every request that reaches the model is the person's.

use crate::app::AppKey;
use crate::ids::{OutputKey, WindowId};
use crate::space::SpaceIndex;
use casement_protocols::vocab::{Assign, PlaceTarget, Toggle};
use porter_core::SpaceId;

/// One management request on the desktop.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DesktopRequest {
    /// Something asked of an app.
    App {
        /// The app.
        app: AppKey,
        /// What was asked.
        ask: AppAsk,
    },
    /// Something asked of a window.
    Window {
        /// The window.
        window: WindowId,
        /// What was asked.
        ask: WindowAsk,
    },
    /// Something asked of a Space.
    Space {
        /// The Space.
        space: SpaceId,
        /// What was asked.
        ask: SpaceAsk,
    },
    /// A new Space at the end of this output's row (`create_space`).
    CreateSpace {
        /// The output.
        output: OutputKey,
    },
}

/// A request on `quire_desktop_app_v1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AppAsk {
    /// Bring the app forward and raise its windows.
    Activate,
    /// Hide the app.
    Hide,
    /// Show the hidden app.
    Unhide,
    /// Hide every other app.
    HideOthers,
    /// Ask the app to quit (every window is asked to close).
    Quit,
    /// End the app's processes (the quit machine, P3).
    ForceQuit,
    /// Activate an app with no windows (answered to the shell, P4).
    Reopen,
}

/// A request on `quire_desktop_window_v1`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum WindowAsk {
    /// Make the window key.
    Activate,
    /// Ask the window to close.
    Close,
    /// Minimise it into the dock.
    Minimise,
    /// Bring it back from the dock.
    Unminimise,
    /// Place it (halves, quarters, centre, fill, restore).
    Place(PlaceTarget),
    /// Enter or leave full screen.
    FullScreen(Toggle),
    /// Move it to a Space.
    ToSpace(SpaceId),
    /// Make a new desktop Space at the end of this output's row and move it there.
    ToNewSpace(OutputKey),
    /// Assign it to every Space, this one, or none.
    Assign(Assign),
}

/// A request on `quire_desktop_space_v1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SpaceAsk {
    /// Switch to the Space.
    Activate,
    /// Remove it, its windows moving to the neighbouring desktop Space.
    Remove,
    /// Move it to this place in its output's row.
    MoveTo(SpaceIndex),
}
