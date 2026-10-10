//! A bound manager's side of publishing (P2-Q1): the objects it has, what each was last told,
//! and the functions that bring one object, or the active pair, up to date. A manager that
//! sent `stop` keeps the objects it has and gets no new ones.

use super::facts::{ActiveFacts, AppFacts, Objects, SpaceFacts, WindowFacts};
use super::{AppData, Bound, DesktopView, Makers, SpaceData, Stopped, WindowData};
use casement_core::WindowId;
use casement_core::app::AppKey;
use casement_core::desktop::DesktopRef;
use casement_protocols::server::quire_desktop_v1::quire_desktop_manager_v1::ActorKind;
use casement_protocols::server::quire_desktop_v1::quire_desktop_window_v1::QuireDesktopWindowV1;
use porter_core::SpaceId;
use smithay::reexports::wayland_server::Resource;
use std::collections::BTreeMap;

/// What each of a manager's objects was last told.
#[derive(Debug, Default)]
pub struct Told {
    apps: BTreeMap<AppKey, AppFacts>,
    windows: BTreeMap<WindowId, WindowFacts>,
    spaces: BTreeMap<SpaceId, SpaceFacts>,
    active: Option<ActiveFacts>,
}

/// A window object the client asked for with `get_window_for`, and what it was told.
#[derive(Debug)]
pub struct Asked {
    window: WindowId,
    object: QuireDesktopWindowV1,
    told: Option<WindowFacts>,
}

/// What publishing to one manager did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Outcome {
    /// Nothing was sent.
    Same,
    /// An object already there was sent its changes and its `done`.
    Changed,
    /// An object was created (and announced on the manager).
    Created,
    /// An object ended.
    Ended,
    /// The manager's `active` was sent.
    Active,
}

impl Outcome {
    /// The outcome of bringing an object up to date: `Created` when it was made, else whether
    /// `sent` events went.
    fn after(made: Outcome, sent: usize) -> Outcome {
        match (made, sent) {
            (Outcome::Created, _) => Outcome::Created,
            (_, 0) => Outcome::Same,
            _ => Outcome::Changed,
        }
    }

    /// Whether the manager's own burst must be closed with `done`.
    pub(super) fn closes_a_burst(self) -> bool {
        matches!(self, Outcome::Created | Outcome::Ended | Outcome::Active)
    }

    /// How much an outcome matters to the manager.
    fn rank(self) -> u8 {
        match self {
            Outcome::Same => 0,
            Outcome::Changed => 1,
            Outcome::Created | Outcome::Ended | Outcome::Active => 2,
        }
    }

    /// The strongest of several outcomes (a created object outranks a changed one).
    pub(super) fn strongest(all: impl IntoIterator<Item = Outcome>) -> Outcome {
        all.into_iter()
            .max_by_key(|outcome| outcome.rank())
            .unwrap_or(Outcome::Same)
    }
}

impl Bound {
    /// A manager just bound: it has no objects and has been told nothing.
    pub(super) fn new(manager: super::QuireDesktopManagerV1, standing: super::Standing) -> Bound {
        Bound {
            manager,
            standing,
            apps: BTreeMap::new(),
            windows: BTreeMap::new(),
            spaces: BTreeMap::new(),
            stopped: Stopped::No,
            asked: Vec::new(),
            told: Told::default(),
        }
    }

    /// Forgets the objects the client destroyed.
    pub(super) fn prune(&mut self) {
        let Bound {
            apps,
            windows,
            spaces,
            asked,
            told,
            ..
        } = self;
        apps.retain(|_, object| object.is_alive());
        windows.retain(|_, object| object.is_alive());
        spaces.retain(|_, object| object.is_alive());
        asked.retain(|held| held.object.is_alive());
        told.apps.retain(|key, _| apps.contains_key(key));
        told.windows.retain(|id, _| windows.contains_key(id));
        told.spaces.retain(|id, _| spaces.contains_key(id));
    }

    /// The Space `id` as it is now: created and announced when this manager lacks it and may
    /// have it, then sent what changed.
    pub(super) fn space(
        &mut self,
        makers: &Makers,
        view: &DesktopView<'_>,
        id: &SpaceId,
    ) -> Outcome {
        let Some(now) = SpaceFacts::of(view, id) else {
            return Outcome::Same;
        };
        let made = match (self.spaces.contains_key(id), self.stopped) {
            (true, _) => Outcome::Same,
            (false, Stopped::Yes) => return Outcome::Same,
            (false, Stopped::No) => {
                let data = SpaceData {
                    space: id.clone(),
                    standing: self.standing,
                };
                let Some(object) = makers.make_space(&self.manager, data) else {
                    return Outcome::Same;
                };
                self.spaces.insert(id.clone(), object);
                Outcome::Created
            }
        };
        let Some(object) = self.spaces.get(id) else {
            return Outcome::Same;
        };
        let sent = now.send(self.told.spaces.get(id), object, view);
        if sent > 0 {
            object.done();
        }
        self.told.spaces.insert(id.clone(), now);
        Outcome::after(made, sent)
    }

    /// The app `key` as it is now (as [`Bound::space`] does a Space).
    pub(super) fn app(&mut self, makers: &Makers, view: &DesktopView<'_>, key: &AppKey) -> Outcome {
        let Some(now) = AppFacts::of(view, key) else {
            return Outcome::Same;
        };
        let made = match (self.apps.contains_key(key), self.stopped) {
            (true, _) => Outcome::Same,
            (false, Stopped::Yes) => return Outcome::Same,
            (false, Stopped::No) => {
                let data = AppData {
                    app: key.clone(),
                    standing: self.standing,
                };
                let Some(object) = makers.make_app(&self.manager, data) else {
                    return Outcome::Same;
                };
                self.apps.insert(key.clone(), object);
                Outcome::Created
            }
        };
        let Some(object) = self.apps.get(key) else {
            return Outcome::Same;
        };
        let sent = now.send(self.told.apps.get(key), object);
        if sent > 0 {
            object.done();
        }
        self.told.apps.insert(key.clone(), now);
        Outcome::after(made, sent)
    }

    /// The window object of `id`, made and announced when this manager lacks it and may have
    /// it; no state is sent (a bind announces every window first, so a window's `parent` can
    /// name any of them).
    pub(super) fn announce_window(&mut self, makers: &Makers, id: WindowId) {
        if self.windows.contains_key(&id) || self.stopped == Stopped::Yes {
            return;
        }
        let data = WindowData {
            window: Some(id),
            standing: self.standing,
        };
        if let Some(object) = makers.make_window(&self.manager, data) {
            self.windows.insert(id, object);
        }
    }

    /// The objects window `now` names that this manager lacks are made first, so the window's
    /// `app` and `place` can name them.
    fn make_what_it_names(
        &mut self,
        makers: &Makers,
        view: &DesktopView<'_>,
        now: &WindowFacts,
    ) -> Outcome {
        let app = match self.apps.contains_key(now.app()) {
            true => Outcome::Same,
            false => self.app(makers, view, now.app()),
        };
        let space = match now.space() {
            Some(id) if !self.spaces.contains_key(id) => self.space(makers, view, id),
            Some(_) | None => Outcome::Same,
        };
        Outcome::strongest([app, space])
    }

    /// The window `id` as it is now, on its own object and on every object the client asked
    /// for with `get_window_for`.
    pub(super) fn window(
        &mut self,
        makers: &Makers,
        view: &DesktopView<'_>,
        id: WindowId,
    ) -> Outcome {
        let Some(now) = WindowFacts::of(view, id) else {
            return Outcome::Same;
        };
        let named = self.make_what_it_names(makers, view, &now);
        let announced = self.windows.contains_key(&id);
        self.announce_window(makers, id);
        let made = match (announced, self.windows.contains_key(&id)) {
            (false, true) => Outcome::Created,
            _ => Outcome::Same,
        };
        let near = Objects {
            apps: &self.apps,
            spaces: &self.spaces,
            windows: &self.windows,
        };
        let mut sent = 0;
        if let Some(object) = self.windows.get(&id) {
            let count = now.send(self.told.windows.get(&id), object, near, view);
            if count > 0 {
                object.done();
            }
            sent += count;
        }
        for held in self.asked.iter_mut().filter(|held| held.window == id) {
            let count = now.send(held.told.as_ref(), &held.object, near, view);
            if count > 0 {
                held.object.done();
            }
            held.told = Some(now.clone());
            sent += count;
        }
        if self.windows.contains_key(&id) {
            self.told.windows.insert(id, now);
        }
        Outcome::strongest([named, made, Outcome::after(Outcome::Same, sent)])
    }

    /// `get_window_for`: `object` is a second view of window `id`; it is sent its burst, and
    /// closed at once when the model has no such window.
    pub(super) fn ask_window(
        &mut self,
        makers: &Makers,
        view: &DesktopView<'_>,
        id: WindowId,
        object: QuireDesktopWindowV1,
    ) -> Outcome {
        let Some(now) = WindowFacts::of(view, id) else {
            object.closed();
            return Outcome::Same;
        };
        let named = self.make_what_it_names(makers, view, &now);
        let near = Objects {
            apps: &self.apps,
            spaces: &self.spaces,
            windows: &self.windows,
        };
        if now.send(None, &object, near, view) > 0 {
            object.done();
        }
        self.asked.push(Asked {
            window: id,
            object,
            told: Some(now),
        });
        named
    }

    /// The active app and key window, sent when they differ from what was told.
    pub(super) fn active(&mut self, view: &DesktopView<'_>) -> Outcome {
        let now = ActiveFacts::of(view);
        if self.told.active.as_ref() == Some(&now) {
            return Outcome::Same;
        }
        let near = Objects {
            apps: &self.apps,
            spaces: &self.spaces,
            windows: &self.windows,
        };
        let (app, key) = now.objects(near);
        self.manager.active(app, key, ActorKind::Unknown);
        self.told.active = Some(now);
        Outcome::Active
    }

    /// The object `gone` names ends: its client is told `closed` (`removed` for a Space).
    pub(super) fn removed(&mut self, gone: &DesktopRef) -> Outcome {
        let ended = match gone {
            DesktopRef::App(key) => {
                self.told.apps.remove(key);
                self.apps.remove(key).map(|object| object.closed())
            }
            DesktopRef::Window(id) => {
                self.told.windows.remove(id);
                let (ended, kept): (Vec<Asked>, Vec<Asked>) = std::mem::take(&mut self.asked)
                    .into_iter()
                    .partition(|held| held.window == *id);
                self.asked = kept;
                ended.iter().for_each(|held| held.object.closed());
                let own = self.windows.remove(id).map(|object| object.closed());
                own.or_else(|| ended.first().map(|_| ()))
            }
            DesktopRef::Space(id) => {
                self.told.spaces.remove(id);
                self.spaces.remove(id).map(|object| object.removed())
            }
            _ => None,
        };
        match ended {
            Some(()) => Outcome::Ended,
            None => Outcome::Same,
        }
    }
}
