//! The focus state, what moves it, and what it asks for.

use super::{Press, PressRoute};
use crate::app::AppKey;
use crate::ids::{OutputKey, WindowId};
use casement_protocols::vocab::{ActorKind, Attention, Emphasis};
use porter_core::SpaceId;

/// Who is active and which windows are key and main (03 A1, A2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Focus {
    /// The active app (it may have no window, A1).
    pub active: Option<AppKey>,
    /// The window that takes the keyboard.
    pub key: Option<WindowId>,
    /// The document the key window acts on.
    pub main: Option<WindowId>,
    /// Apps, most recently active first.
    pub app_mru: Vec<AppKey>,
    /// Windows, most recently key first.
    pub window_mru: Vec<WindowId>,
}

impl Focus {
    /// Nothing active, nothing key.
    pub fn none() -> Focus {
        Focus {
            active: None,
            key: None,
            main: None,
            app_mru: Vec::new(),
            window_mru: Vec::new(),
        }
    }
}

/// Whether an xdg-activation token checked out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenCheck {
    /// Issued for a recent input of the focused client.
    Valid,
    /// Issued, but no longer good (too old, or focus moved since).
    Stale,
    /// None given.
    Missing,
}

/// What moves focus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FocusIn {
    /// A press on a window.
    Press(Press),
    /// A window asked for activation with a token.
    Token {
        /// The window.
        window: WindowId,
        /// How the token checked out.
        token: TokenCheck,
    },
    /// A window mapped.
    Mapped(WindowId),
    /// A window unmapped.
    Unmapped(WindowId),
    /// The switcher, the dock, the launcher or an intent activated an app.
    ActivateApp {
        /// The app.
        app: AppKey,
        /// Who asked.
        by: ActorKind,
    },
    /// An assistant run's live lease keys the window it holds. Not an app's activation: the
    /// actor is the person's own request carried by the lease, so A7 still refuses every app.
    LeaseFocus {
        /// The window the lease holds.
        window: WindowId,
    },
    /// An app was hidden.
    HideApp(AppKey),
    /// An app was unhidden.
    UnhideApp(AppKey),
    /// An output began showing a Space.
    SpaceShown {
        /// The output.
        output: OutputKey,
        /// The Space.
        space: SpaceId,
    },
    /// A sheet opened on a window.
    SheetOpened {
        /// The window it covers.
        parent: WindowId,
        /// The sheet.
        sheet: WindowId,
    },
    /// A window's sheet closed.
    SheetClosed {
        /// The window it covered.
        parent: WindowId,
    },
}

/// What focus asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FocusOut {
    /// Give the keyboard to this window, or to nothing.
    Keyboard(Option<WindowId>),
    /// Raise these windows, bottom first.
    Raise(Vec<WindowId>),
    /// Route the press this way.
    Route(PressRoute),
    /// These windows' emphasis changed.
    Emphasis(Vec<(WindowId, Emphasis)>),
    /// The active app changed.
    ActiveApp(Option<AppKey>),
    /// An app asks for attention (A7: a window mapped without a valid token).
    Attention(AppKey, Attention),
    /// Ring the system bell for this app (a press on a window behind its sheet).
    Bell(AppKey),
}
