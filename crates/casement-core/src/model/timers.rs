//! Timers the model hears of, replies from clients, and the settings.

use super::Model;
use super::moved::Moved;
use crate::app::{AppKey, AppLife, PING_DEADLINE, PING_GAP, Pinging, ping_plan};
use crate::cause::Cause;
use crate::effect::{CoreNote, Effect, Held};
use crate::ids::ClientKey;
use crate::settings::CasementSettings;
use crate::time::{Due, TimerKey};
use ds_behaviour::space_swipe::SwipeIn;

impl Model {
    /// A timer fired. It leaves the book; a ping unanswered marks its client's app as not
    /// responding. A timer for something that is gone is noted.
    pub(super) fn fire(self, key: TimerKey) -> Moved {
        let mut model = self;
        model.timers = model.timers.cancel(&key);
        let Some(gone) = model.is_gone(&key) else {
            return Moved::of(model);
        };
        match (gone, &key) {
            (Gone::Yes, _) => Moved::of(model).with(Effect::Note(CoreNote::StaleTimer(key))),
            (Gone::No, TimerKey::Ping(client)) => model.not_responding(*client),
            (Gone::No, TimerKey::QuitGrace(app)) => model.quit_grace_over(app),
            (Gone::No, TimerKey::LockWait) => model.lock_wait_over(),
            (Gone::No, TimerKey::OverviewSettle) => model.overview_settled(),
            (Gone::No, TimerKey::SpaceFinish(_)) => model.space_swipe(SwipeIn::Elapsed),
            (Gone::No, _) => Moved::of(model),
        }
    }

    /// Whether what `key` is for is gone; none for a timer the model does not track.
    fn is_gone(&self, key: &TimerKey) -> Option<Gone> {
        let exists = |found: Option<()>| found.map_or(Gone::Yes, |()| Gone::No);
        match key {
            TimerKey::Ping(client) | TimerKey::PingGap(client) => {
                Some(exists(self.clients.get(client).map(|_| ())))
            }
            TimerKey::QuitGrace(app) => Some(exists(self.apps.get(app).map(|_| ()))),
            TimerKey::FrameWatch(output) | TimerKey::SpaceFinish(output) => {
                Some(exists(self.outputs.0.get(output).map(|_| ())))
            }
            TimerKey::KeyRepeat(_)
            | TimerKey::ImeHold
            | TimerKey::IdleNotify(..)
            | TimerKey::SwitcherDelay
            | TimerKey::HotCorner(_)
            | TimerKey::LightDwell
            | TimerKey::TrustedArm(_)
            | TimerKey::LeaseYield(_) => None,
            TimerKey::LockWait | TimerKey::OverviewSettle => Some(Gone::No),
        }
    }

    /// Input arrived: the client of the key window is pinged, unless a ping is out or one went
    /// a moment ago. A client that does not answer within [`PING_DEADLINE`] is not responding.
    pub(super) fn input_seen(self) -> Moved {
        let keyed = self
            .focus
            .key
            .and_then(|id| self.windows.get(&id))
            .map(|window| (window.client, window.app.clone()));
        match keyed {
            Some((client, app)) => self.pinged(client, &app),
            None => Moved::of(self),
        }
    }

    /// `client` is pinged when [`ping_plan`] says one is due, by the life of `app` (the key
    /// window's); the deadline and the gap are armed here and in the book, so the next input
    /// sees them.
    fn pinged(self, client: ClientKey, app: &AppKey) -> Moved {
        let life = self.apps.get(app).map(|record| record.life.clone());
        let held = |key: TimerKey| match self.timers.is_armed(&key) {
            true => Held::Yes,
            false => Held::No,
        };
        let plan = ping_plan(
            life.as_ref(),
            held(TimerKey::Ping(client)),
            held(TimerKey::PingGap(client)),
        );
        match plan {
            Pinging::Hold => Moved::of(self),
            Pinging::Send => {
                let deadline = Due {
                    at: self.now.after(PING_DEADLINE),
                    key: TimerKey::Ping(client),
                };
                let gap = Due {
                    at: self.now.after(PING_GAP),
                    key: TimerKey::PingGap(client),
                };
                let mut model = self;
                model.timers = model.timers.arm(deadline.clone()).arm(gap.clone());
                Moved::of(model)
                    .with(Effect::Ping { client })
                    .with(Effect::ArmTimer(deadline))
                    .with(Effect::ArmTimer(gap))
            }
        }
    }

    /// The client's apps stopped answering: it is one connection, so they all did.
    fn not_responding(self, client: ClientKey) -> Moved {
        let since = self.now;
        let apps = self.clients.get(&client).cloned().unwrap_or_default();
        apps.into_iter().fold(Moved::of(self), |moved, app| {
            moved.then(|model| {
                model.with_life(Some(&app), |life| match life {
                    AppLife::Running | AppLife::Windowless => AppLife::NotResponding { since },
                    other => other.clone(),
                })
            })
        })
    }

    /// A client answered a ping: the timer is cancelled, and its apps respond again.
    pub(super) fn pong(self, client: ClientKey) -> Moved {
        let apps = self.clients.get(&client).cloned().unwrap_or_default();
        let mut model = self;
        model.timers = model.timers.cancel(&TimerKey::Ping(client));
        apps.into_iter().fold(
            Moved::of(model).with(Effect::CancelTimer(TimerKey::Ping(client))),
            |moved, app| {
                moved.then(|model| {
                    let windows = model
                        .apps
                        .get(&app)
                        .map_or(0, |record| record.windows.len());
                    model.with_life(Some(&app), |life| match (life, windows) {
                        (AppLife::NotResponding { .. }, 0) => AppLife::Windowless,
                        (AppLife::NotResponding { .. }, _) => AppLife::Running,
                        (other, _) => other.clone(),
                    })
                })
            },
        )
    }

    /// The app's life becomes what `next` says of its current one.
    fn with_life(self, app: Option<&AppKey>, next: impl FnOnce(&AppLife) -> AppLife) -> Moved {
        let mut model = self;
        if let Some(record) = app.and_then(|app| model.apps.get_mut(app)) {
            record.life = next(&record.life);
        }
        Moved::of(model)
    }

    /// The settings changed: windows are laid out again under them.
    pub(super) fn change_settings(self, settings: CasementSettings) -> Moved {
        if settings == self.settings {
            return Moved::of(self);
        }
        let outputs = self.outputs.0.keys().cloned().collect();
        Model { settings, ..self }
            .redrawn(Cause::Configure, outputs)
            .then(Model::night_drive)
    }
}

/// Whether the thing a timer was armed for still exists.
enum Gone {
    Yes,
    No,
}
