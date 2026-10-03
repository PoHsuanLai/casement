//! What the person does that is not a toplevel's own request: activating an app, a press,
//! a desktop command a key binding resolved to, and which targets an input may reach.
//!
//! These are the model's entry points for what `Event` has no variant for: a press already
//! hit-tested to a window, an app activation, a command a binding resolved to. The edge calls
//! them as it calls `Model::step`. While the session is locked none of them moves anything.

use super::Model;
use super::moved::Moved;
use crate::app::{AppKey, AppVisibility};
use crate::effect::Effect;
use crate::focus::{FocusIn, Press};
use crate::ids::WindowId;
use crate::input::{DesktopCommand, InputTarget, InputUse, Origin, Reach, ReachFacts, reach};
use crate::lock::LockState;
use crate::settings::Enabled;
use crate::time::Mono;
use crate::window::{SpacePlace, Window, WindowState};
use casement_protocols::vocab::ActorKind;

impl Model {
    /// Whether an input from `origin` may reach `target` for `use_` now (the lock, the agent,
    /// the feed and trusted surfaces: the reach table over this model's lock).
    pub fn reach(&self, origin: &Origin, target: &InputTarget, use_: InputUse) -> Reach {
        reach(origin, target, use_, &ReachFacts { lock: &self.lock })
    }

    /// A press on a window, routed before any client sees it (03 §1.2). The effects say how:
    /// a [`Effect::Route`], and a raise and the keyboard when it activates. A press on a layer
    /// surface never comes here: layer surfaces always receive their presses and take the
    /// keyboard only by their own interactivity.
    pub fn press(self, press: Press, now: Mono) -> (Model, Vec<Effect>) {
        self.run(now, |model| {
            let seen = model
                .windows
                .get(&press.window)
                .is_some_and(|w| model.is_seen(w));
            match (&model.lock, seen) {
                (LockState::Unlocked, true) => model.feed(FocusIn::Press(press)),
                _ => Moved::of(model),
            }
        })
    }

    /// `app` is activated (the switcher, the dock, the launcher, an intent). An app hidden
    /// comes back first. When the app has no window on a Space an output shows, the output
    /// that shows the Space of its most recent window switches to it and focus keys that window
    /// (rule S5), unless `spaces.switch_on_activate` is off.
    pub fn activate_app(self, app: AppKey, by: ActorKind, now: Mono) -> (Model, Vec<Effect>) {
        self.run(now, |model| match model.lock {
            LockState::Unlocked => model.activation(app, by),
            _ => Moved::of(model),
        })
    }

    /// A desktop command a key binding resolved to, run for the person at the keyboard.
    pub fn command(self, command: DesktopCommand, now: Mono) -> (Model, Vec<Effect>) {
        self.run(now, |model| match model.lock {
            LockState::Unlocked => model.carry_out_command(command),
            _ => Moved::of(model),
        })
    }

    fn activation(self, app: AppKey, by: ActorKind) -> Moved {
        let refused = matches!(
            by,
            ActorKind::App | ActorKind::ThirdParty | ActorKind::Unknown
        );
        if refused {
            return self.feed(FocusIn::ActivateApp { app, by });
        }
        let follow = self.settings.spaces.switch_on_activate;
        let next = app.clone();
        self.unhide_app(&app)
            .then(|model| model.feed(FocusIn::ActivateApp { app, by }))
            .then(|model| match (model.focus.key, follow) {
                (None, Enabled::On) => model.follow_to_a_window_of(&next),
                _ => Moved::of(model),
            })
    }

    /// Rule S5: the app is active with no key window, so show the Space of its most recent
    /// window that is not in the dock, if it has one on a Space not shown.
    fn follow_to_a_window_of(self, app: &AppKey) -> Moved {
        let held: Vec<&Window> = self
            .apps
            .get(app)
            .map(|record| record.windows.as_slice())
            .unwrap_or_default()
            .iter()
            .filter_map(|id| self.windows.get(id))
            .filter(|window| !matches!(window.state, WindowState::Minimised { .. }))
            .collect();
        let latest = self
            .focus
            .window_mru
            .iter()
            .find_map(|id| held.iter().find(|w| w.id == *id))
            .or_else(|| held.last());
        let space = latest.and_then(|window| match &window.place {
            SpacePlace::On(space) => Some(space.clone()),
            SpacePlace::AllSpaces => None,
        });
        match space {
            Some(space) => self.show(&space),
            None => Moved::of(self),
        }
    }

    fn carry_out_command(self, command: DesktopCommand) -> Moved {
        let key = self.focus.key.and_then(|id| self.windows.get(&id)).cloned();
        let active = self.focus.active.clone();
        match command {
            DesktopCommand::CloseKey => match key {
                Some(window) => Moved::of(self).with(Effect::SendClose { window: window.id }),
                None => Moved::of(self),
            },
            DesktopCommand::MinimiseKey => match key {
                Some(window) => self.minimise(&window),
                None => Moved::of(self),
            },
            DesktopCommand::HideActive => match active {
                Some(app) => self.hide_app(&app),
                None => Moved::of(self),
            },
            DesktopCommand::HideOthers => self.hide_others(active),
            DesktopCommand::QuitActive => self.close_all_of(active),
            DesktopCommand::ForceQuitPanel => Moved::of(self),
            DesktopCommand::SwitchSpace(step) => match self.working_output() {
                Some(output) => self.switch(&output, step),
                None => Moved::of(self),
            },
            DesktopCommand::Place(target) => match key {
                Some(window) => self.place(&window, target),
                None => Moved::of(self),
            },
        }
    }

    /// Every shown app but `keep` is hidden.
    fn hide_others(self, keep: Option<AppKey>) -> Moved {
        let others: Vec<AppKey> = self
            .apps
            .values()
            .filter(|app| Some(&app.key) != keep.as_ref())
            .filter(|app| matches!(app.visibility, AppVisibility::Shown))
            .map(|app| app.key.clone())
            .collect();
        others.into_iter().fold(Moved::of(self), |moved, app| {
            moved.then(|m| m.hide_app(&app))
        })
    }

    /// Every window of the app is asked to close (the polite half of quitting).
    fn close_all_of(self, app: Option<AppKey>) -> Moved {
        let windows: Vec<WindowId> = app
            .and_then(|app| self.apps.get(&app).map(|record| record.windows.clone()))
            .unwrap_or_default();
        let closes = windows
            .into_iter()
            .map(|window| Effect::SendClose { window });
        Moved::of(self).with_all(closes)
    }
}
