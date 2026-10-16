//! What the focus machine reads of the model.

use crate::app::{App, AppKey, AppVisibility};
use crate::ids::WindowId;
use crate::space::SpaceBook;
use crate::window::{Parent, SpacePlace, Window, WindowState};
use casement_protocols::vocab::Modality;
use std::collections::BTreeMap;

/// The model's windows, apps and Spaces, borrowed for one focus step. A step reads the tables
/// as they stand after the event's own change: a mapped window is in `windows`, an unmapped
/// one is gone, an output already shows the Space it was just given.
///
/// `apps` is a departure from the P0-Z1 freeze (FINDINGS): the hide row needs to know which
/// apps are hidden, and the activation row takes an app's windows in the app's own order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FocusView<'a> {
    /// Every window.
    pub windows: &'a BTreeMap<WindowId, Window>,
    /// Every Space.
    pub spaces: &'a SpaceBook,
    /// Every app.
    pub apps: &'a BTreeMap<AppKey, App>,
}

impl FocusView<'_> {
    /// The window with this id.
    pub(super) fn window(&self, id: WindowId) -> Option<&Window> {
        self.windows.get(&id)
    }

    /// Whether the app is hidden (Cmd+H).
    pub(super) fn app_hidden(&self, app: &AppKey) -> bool {
        self.apps
            .get(app)
            .is_some_and(|app| matches!(app.visibility, AppVisibility::Hidden { .. }))
    }

    /// Whether the window is on a Space an output shows (or on every Space) and not in the
    /// dock. Says nothing of its app being hidden: activating a hidden app unhides it.
    pub(super) fn is_shown(&self, window: &Window) -> bool {
        let on_shown_space = match &window.place {
            SpacePlace::AllSpaces => true,
            SpacePlace::On(id) => self.spaces.get(id).is_some_and(|space| {
                self.spaces
                    .shown_on(&space.output)
                    .is_some_and(|shown| shown.id == space.id)
            }),
        };
        on_shown_space && !matches!(window.state, WindowState::Minimised { .. })
    }

    /// Whether the person can see the window: shown, and its app not hidden.
    pub fn is_visible(&self, window: &Window) -> bool {
        self.is_shown(window) && !self.app_hidden(&window.app)
    }

    /// Every window of `app` in the app's own order, bottom first: the app's record when it
    /// has one, else by id.
    pub(super) fn windows_of(&self, app: &AppKey) -> Vec<&Window> {
        match self.apps.get(app) {
            Some(record) => record
                .windows
                .iter()
                .filter_map(|id| self.window(*id))
                .collect(),
            None => self.windows.values().filter(|w| w.app == *app).collect(),
        }
    }

    /// The modal sheet open on `window`, if any: one that is shown. A minimised or off-Space
    /// sheet cannot take the key, so its window is keyed as if it had none.
    pub(super) fn sheet_of(&self, window: WindowId) -> Option<&Window> {
        self.windows.values().find(|candidate| {
            candidate.parent == Parent::Window(window)
                && candidate.modality == Modality::WindowModal
                && self.is_shown(candidate)
        })
    }
}
