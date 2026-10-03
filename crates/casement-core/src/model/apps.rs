//! App records: created with an app's first window, hidden and unhidden as a group, gone
//! with the app's last client.

use super::Model;
use super::moved::Moved;
use crate::app::{App, AppKey, AppLife, AppVisibility, PidSet};
use crate::cause::Cause;
use crate::effect::Effect;
use crate::focus::FocusIn;
use crate::ids::{OutputKey, WindowId};
use casement_protocols::vocab::Attention;
use std::collections::BTreeSet;

impl Model {
    /// The app's record, created shown and running with no windows when the app is new.
    pub(super) fn with_app(self, key: &AppKey) -> Model {
        let mut apps = self.apps;
        apps.entry(key.clone()).or_insert_with(|| App {
            key: key.clone(),
            pids: PidSet::default(),
            windows: Vec::new(),
            life: AppLife::Running,
            visibility: AppVisibility::Shown,
            attention: Attention::None,
        });
        Model { apps, ..self }
    }

    /// `window` joins the top of its app's own order; the app is running.
    pub(super) fn with_app_window(self, key: &AppKey, window: WindowId) -> Model {
        let mut model = self.with_app(key);
        if let Some(app) = model.apps.get_mut(key) {
            app.windows.push(window);
            app.life = life_with(&app.life, app.windows.len());
        }
        model
    }

    /// `window` leaves its app's order and its hidden restore order; an app left with no
    /// window is windowless (it still runs, 03 A1).
    pub(super) fn without_app_window(self, key: &AppKey, window: WindowId) -> Model {
        let mut model = self;
        if let Some(app) = model.apps.get_mut(key) {
            app.windows.retain(|held| *held != window);
            if let AppVisibility::Hidden { restore_order } = &mut app.visibility {
                restore_order.retain(|held| *held != window);
            }
            app.life = life_with(&app.life, app.windows.len());
        }
        model
    }

    /// The app is hidden (Cmd+H): its windows leave the desktop until it is unhidden.
    pub(super) fn hide_app(self, key: &AppKey) -> Moved {
        let Some(app) = self.apps.get(key) else {
            return Moved::of(self);
        };
        if matches!(app.visibility, AppVisibility::Hidden { .. }) {
            return Moved::of(self);
        }
        let order = app.windows.clone();
        let mut model = self;
        if let Some(app) = model.apps.get_mut(key) {
            app.visibility = AppVisibility::Hidden {
                restore_order: order.clone(),
            };
        }
        let outputs = model.outputs_of(&order);
        Moved::of(model)
            .then(|model| model.feed(FocusIn::HideApp(key.clone())))
            .then(|model| model.redrawn(Cause::Configure, outputs))
    }

    /// The app is shown again, its windows back in the order they had.
    pub(super) fn unhide_app(self, key: &AppKey) -> Moved {
        let Some(AppVisibility::Hidden { restore_order }) =
            self.apps.get(key).map(|app| app.visibility.clone())
        else {
            return Moved::of(self);
        };
        let mut model = self;
        if let Some(app) = model.apps.get_mut(key) {
            app.visibility = AppVisibility::Shown;
        }
        let outputs = model.outputs_of(&restore_order);
        Moved::of(model)
            .then(|model| model.raised(&restore_order))
            .then(|model| model.feed(FocusIn::UnhideApp(key.clone())))
            .then(|model| model.redrawn(Cause::Configure, outputs))
    }

    /// The app is gone: its record leaves the table and focus moves off it.
    pub(super) fn drop_app(self, key: &AppKey) -> Moved {
        let active = self.focus.active.as_ref() == Some(key);
        let moved = match active {
            true => self.feed(FocusIn::HideApp(key.clone())),
            false => Moved::of(self),
        };
        moved.then(|mut model| {
            model.apps.remove(key);
            model.focus.app_mru.retain(|app| app != key);
            if model.focus.active.as_ref() == Some(key) {
                model.focus.active = None;
            }
            Moved::of(model)
        })
    }

    /// The outputs these windows are on, once each, in key order.
    pub(super) fn outputs_of(&self, windows: &[WindowId]) -> Vec<OutputKey> {
        windows
            .iter()
            .filter_map(|id| self.windows.get(id))
            .filter_map(|window| self.output_of(window))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    /// The scene is laid out again for `cause` and each output asked for a frame.
    pub(super) fn redrawn(self, cause: Cause, outputs: Vec<OutputKey>) -> Moved {
        Moved::of(self)
            .with(Effect::Relayout(cause))
            .with_all(outputs.into_iter().map(Effect::RequestFrames))
    }
}

/// An app's life once it has (or has not) windows: a running or windowless app follows its
/// windows; one that is quitting or not answering stays so.
fn life_with(life: &AppLife, windows: usize) -> AppLife {
    match (life, windows) {
        (AppLife::Launching { .. } | AppLife::Running | AppLife::Windowless, 0) => {
            AppLife::Windowless
        }
        (AppLife::Launching { .. } | AppLife::Running | AppLife::Windowless, _) => AppLife::Running,
        (AppLife::Quitting(_) | AppLife::NotResponding { .. }, _) => life.clone(),
    }
}
