//! App records: created with an app's first window, hidden and unhidden as a group, gone
//! with the app's last client.

use super::Model;
use super::moved::Moved;
use crate::app::{
    App, AppKey, AppLife, AppVisibility, KILL_GRACE, PidSet, QUIT_GRACE, QuitPhase, StopSignal,
    Termination, force_quit_rows,
};
use crate::cause::Cause;
use crate::effect::Effect;
use crate::focus::FocusIn;
use crate::ids::{ClientKey, OutputKey, WindowId};
use crate::shell::ShellNote;
use crate::time::{Due, Mono, TimerKey};
use crate::window::WindowOrigin;
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

    /// The app is asked to quit (Cmd+Q, or the shell): every window is asked to close, and the
    /// app shows as quitting until it goes or [`QUIT_GRACE`] passes with it still here.
    pub(super) fn quit_app(self, app: Option<AppKey>) -> Moved {
        let Some(key) = app.filter(|key| self.apps.contains_key(key)) else {
            return Moved::of(self);
        };
        let since = self.now;
        let mut model = self;
        if let Some(record) = model.apps.get_mut(&key) {
            record.life = asked_to_quit(&record.life, since);
        }
        let grace = Due {
            at: since.after(QUIT_GRACE),
            key: TimerKey::QuitGrace(key.clone()),
        };
        model.timers = model.timers.arm(grace.clone());
        model.close_all_of(Some(key)).with(Effect::ArmTimer(grace))
    }

    /// The app is force quit: its processes get `SIGTERM` now and `SIGKILL` after
    /// [`KILL_GRACE`] if it is still here. Only connections that serve this app and no other
    /// are named (a connection that carries several apps' windows is left alone), and for an
    /// X11 app its own windows (never the satellite's connection, which carries them all).
    pub(super) fn force_quit_app(self, app: &AppKey) -> Moved {
        let stop = self.stop_of(app, StopSignal::Term);
        if stop.is_empty() {
            return Moved::of(self);
        }
        let since = self.now;
        let mut model = self;
        if let Some(record) = model.apps.get_mut(app) {
            record.life = AppLife::Quitting(QuitPhase::Stopping);
        }
        let grace = Due {
            at: since.after(KILL_GRACE),
            key: TimerKey::QuitGrace(app.clone()),
        };
        model.timers = model.timers.arm(grace.clone());
        Moved::of(model)
            .with(Effect::Terminate(stop))
            .with(Effect::ArmTimer(grace))
    }

    /// The grace after a quit request or a `SIGTERM` is over. A quit that left the app here
    /// (a sheet asks to save, or it declined) is over: the app is running again. A stop that
    /// did not end it is made with `SIGKILL`.
    pub(super) fn quit_grace_over(self, app: &AppKey) -> Moved {
        let Some(record) = self.apps.get(app) else {
            return Moved::of(self);
        };
        match record.life.clone() {
            AppLife::Quitting(QuitPhase::Asked { .. }) => {
                let windows = record.windows.len();
                let mut model = self;
                if let Some(record) = model.apps.get_mut(app) {
                    record.life = life_with(&AppLife::Running, windows);
                }
                Moved::of(model)
            }
            AppLife::Quitting(QuitPhase::Stopping) => {
                let stop = self.stop_of(app, StopSignal::Kill);
                Moved::of(self).with(Effect::Terminate(stop))
            }
            _ => Moved::of(self),
        }
    }

    /// The Force Quit Applications window is asked for: the shell is told which apps run and
    /// which of them answer.
    pub(super) fn force_quit_panel(self) -> Moved {
        let rows = force_quit_rows(&self.apps);
        Moved::of(self).with(Effect::Shell(ShellNote::ForceQuitPanel(rows)))
    }

    /// What stopping `app` names: its own connections and its X11 windows.
    fn stop_of(&self, app: &AppKey, signal: StopSignal) -> Termination {
        Termination {
            app: app.clone(),
            clients: self.sole_clients_of(app),
            x11: self.x11_windows_of(app),
            signal,
        }
    }

    /// The connections whose windows all belong to `app`, and that serve it. A connection
    /// that carries X11 windows is the satellite's, shared by every X11 program, and is never
    /// one of these.
    fn sole_clients_of(&self, app: &AppKey) -> Vec<ClientKey> {
        self.clients
            .iter()
            .filter(|(_, served)| served.as_ref() == Some(app))
            .filter(|(client, _)| {
                self.windows
                    .values()
                    .filter(|window| window.client == **client)
                    .all(|window| window.app == *app && window.origin == WindowOrigin::Wayland)
            })
            .map(|(client, _)| *client)
            .collect()
    }

    /// The windows of `app` that are X11 programs' own.
    fn x11_windows_of(&self, app: &AppKey) -> Vec<WindowId> {
        self.windows
            .values()
            .filter(|window| window.app == *app && window.origin == WindowOrigin::X11)
            .map(|window| window.id)
            .collect()
    }

    /// The app is gone: its record leaves the table and focus moves off it.
    pub(super) fn drop_app(self, key: &AppKey) -> Moved {
        let grace = TimerKey::QuitGrace(key.clone());
        let timer = self.timers.is_armed(&grace);
        let moved = self.dropped_app(key);
        match timer {
            true => moved.then(|mut model| {
                model.timers = model.timers.cancel(&grace);
                Moved::of(model).with(Effect::CancelTimer(grace))
            }),
            false => moved,
        }
    }

    fn dropped_app(self, key: &AppKey) -> Moved {
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
pub(super) fn life_with(life: &AppLife, windows: usize) -> AppLife {
    match (life, windows) {
        (AppLife::Launching { .. } | AppLife::Running | AppLife::Windowless, 0) => {
            AppLife::Windowless
        }
        (AppLife::Launching { .. } | AppLife::Running | AppLife::Windowless, _) => AppLife::Running,
        (AppLife::Quitting(_) | AppLife::NotResponding { .. }, _) => life.clone(),
    }
}

/// The life of an app just asked to quit: a quit already under way, or a hung app, stays so.
fn asked_to_quit(life: &AppLife, since: Mono) -> AppLife {
    match life {
        AppLife::Launching { .. } | AppLife::Running | AppLife::Windowless => {
            AppLife::Quitting(QuitPhase::Asked { since })
        }
        AppLife::Quitting(_) | AppLife::NotResponding { .. } => life.clone(),
    }
}
