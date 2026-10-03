//! How a focus takes a window as key, and which window is then main (03 A2).

use super::super::{Focus, FocusView};
use crate::app::AppKey;
use crate::window::{Parent, Window};
use casement_protocols::vocab::{Modality, WindowRole};

/// Whether a window, when key, is also the document the person is working on: a document or
/// a shell window. Utility panels, sheets, dialogs and picture in picture act on a main window
/// that is not themselves.
fn takes_main(window: &Window) -> bool {
    matches!(window.role, WindowRole::Document | WindowRole::Shell)
}

/// Whether a window is a modeless utility: a panel, or a child that is not a sheet. It never
/// takes the key by mapping (03 §1.2, Mapped row).
pub(super) fn is_modeless_utility(window: &Window) -> bool {
    let child_that_blocks_nothing =
        window.parent != Parent::None && window.modality != Modality::WindowModal;
    window.role == WindowRole::Utility || child_that_blocks_nothing
}

impl Focus {
    /// This focus with `window` key, its app active and both at the front of their MRUs.
    pub(super) fn keyed(self, window: &Window, view: &FocusView) -> Focus {
        let main = self.main_for(window, view);
        self.keyed_over(window, main)
    }

    /// The same with `main` named, as a sheet names its window.
    pub(super) fn keyed_over(self, window: &Window, main: Option<crate::ids::WindowId>) -> Focus {
        Focus {
            active: Some(window.app.clone()),
            key: Some(window.id),
            main,
            app_mru: front(self.app_mru, &window.app),
            window_mru: front(self.window_mru, &window.id),
        }
    }

    /// This focus with `app` active and `key` (its window, if it has one to show) key.
    pub(super) fn activated(self, app: &AppKey, key: Option<&Window>, view: &FocusView) -> Focus {
        match key {
            Some(window) => self.keyed(window, view),
            None => Focus {
                active: Some(app.clone()),
                key: None,
                main: None,
                app_mru: front(self.app_mru, app),
                window_mru: self.window_mru,
            },
        }
    }

    /// This focus with no app active and nothing key.
    pub(super) fn idle(self) -> Focus {
        Focus {
            active: None,
            key: None,
            main: None,
            ..self
        }
    }

    /// The window most recently key among `candidates`, else the last (the front of the app's
    /// own order).
    pub(super) fn latest<'w>(&self, candidates: &[&'w Window]) -> Option<&'w Window> {
        self.window_mru
            .iter()
            .find_map(|id| candidates.iter().copied().find(|w| w.id == *id))
            .or_else(|| candidates.last().copied())
    }

    /// The main window while `key` is key: itself when it takes main, else the window it is a
    /// child of, else the main it already had when that is still a window of its app, else the
    /// app's most recent shown window that takes main.
    pub(super) fn main_for(&self, key: &Window, view: &FocusView) -> Option<crate::ids::WindowId> {
        if takes_main(key) {
            return Some(key.id);
        }
        let parent = match key.parent {
            Parent::Window(id) => view.window(id).map(|w| w.id),
            Parent::None => None,
        };
        let kept = self
            .main
            .and_then(|id| view.window(id))
            .filter(|w| w.app == key.app && view.is_shown(w))
            .map(|w| w.id);
        let documents: Vec<&Window> = view
            .windows_of(&key.app)
            .into_iter()
            .filter(|w| takes_main(w) && view.is_shown(w))
            .collect();
        parent
            .or(kept)
            .or_else(|| self.latest(&documents).map(|w| w.id))
    }
}

/// `list` with `item` moved (or put) at the front.
fn front<T: PartialEq + Clone>(list: Vec<T>, item: &T) -> Vec<T> {
    std::iter::once(item.clone())
        .chain(list.into_iter().filter(|x| x != item))
        .collect()
}
