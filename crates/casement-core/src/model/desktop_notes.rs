//! What changed on the desktop in one step, for `quire_desktop_v1` clients (P2-Q1): one
//! snapshot before the step and one comparison after it ask for a `DesktopNote` for every app,
//! window and Space that appeared or changed, for a change of the active app or key window, and
//! for every object that went. No transition has to remember to say so.

use super::Model;
use crate::app::{AppKey, AppVisibility, MenuSource};
use crate::desktop::{DesktopNote, DesktopRef};
use crate::effect::Effect;
use crate::ids::{OutputKey, WindowId};
use crate::space::Space;
use crate::window::{Parent, SpacePlace, WindowState};
use casement_protocols::vocab::{
    AppLifeKind, AppVisibility as Visibility, Attention, Emphasis, Modality, PlacementMark,
    SensitivityLevel, WindowRole,
};
use porter_core::SpaceId;
use std::collections::BTreeMap;

/// What an app shows `quire_desktop_v1` clients.
#[derive(Debug, Clone, PartialEq, Eq)]
struct AppDesk {
    life: AppLifeKind,
    visibility: Visibility,
    attention: Attention,
    rank: usize,
    menu: MenuSource,
}

/// What a window shows `quire_desktop_v1` clients. The whole state is kept, not only its kind:
/// a window that moves is a change.
#[derive(Debug, Clone, PartialEq, Eq)]
struct WindowDesk {
    app: AppKey,
    role: WindowRole,
    modality: Modality,
    place: SpacePlace,
    state: WindowState,
    mark: PlacementMark,
    output: Option<OutputKey>,
    parent: Parent,
    emphasis: Emphasis,
    sensitivity: SensitivityLevel,
    life: AppLifeKind,
    menu_path: String,
}

/// The desktop's publishable state at one instant.
#[derive(Debug, Clone)]
pub(super) struct DeskSnapshot {
    apps: BTreeMap<AppKey, AppDesk>,
    windows: BTreeMap<WindowId, WindowDesk>,
    spaces: BTreeMap<SpaceId, Space>,
    active: (Option<AppKey>, Option<WindowId>),
}

impl Model {
    /// Where the global menu of `app` lives: the key window's when it declared one, else the
    /// first window of the app (in the app's own order) that did, else none. The shell reads
    /// it when the app becomes active.
    pub fn app_menu(&self, app: &AppKey) -> MenuSource {
        let key = self.focus().key;
        let held = self.apps().get(app).map(|held| held.windows.as_slice());
        key.into_iter()
            .chain(held.into_iter().flatten().copied())
            .filter_map(|id| self.windows().get(&id))
            .filter(|window| window.app == *app)
            .map(|window| &window.hints.menu)
            .find(|menu| menu.is_declared())
            .cloned()
            .unwrap_or_default()
    }
}

impl DeskSnapshot {
    /// The state of `model` now.
    pub(super) fn of(model: &Model) -> DeskSnapshot {
        let focus = model.focus();
        // Each app's place in the most-recently-active list, found once (the first place when an
        // app is listed twice) instead of a scan of the list per app.
        let ranks: BTreeMap<&AppKey, usize> = focus
            .app_mru
            .iter()
            .enumerate()
            .rev()
            .map(|(place, held)| (held, place))
            .collect();
        let apps: BTreeMap<AppKey, AppDesk> = model
            .apps()
            .iter()
            .map(|(key, app)| {
                let visibility = match app.visibility {
                    AppVisibility::Shown => Visibility::Shown,
                    AppVisibility::Hidden { .. } => Visibility::Hidden,
                };
                let rank = ranks.get(&key).copied().unwrap_or(focus.app_mru.len());
                let desk = AppDesk {
                    life: app.life.kind(),
                    visibility,
                    attention: app.attention,
                    rank,
                    menu: model.app_menu(key),
                };
                (key.clone(), desk)
            })
            .collect();
        let windows = model
            .windows()
            .values()
            .map(|window| {
                let life = model
                    .apps()
                    .get(&window.app)
                    .map_or(AppLifeKind::Running, |app| app.life.kind());
                let desk = WindowDesk {
                    app: window.app.clone(),
                    role: window.role,
                    modality: window.modality,
                    place: window.place.clone(),
                    state: window.state.clone(),
                    mark: window.state.mark(),
                    output: model.output_of(window),
                    parent: window.parent,
                    emphasis: window.emphasis,
                    sensitivity: window.sensitivity,
                    life,
                    menu_path: window.hints.menu.window_path().to_owned(),
                };
                (window.id, desk)
            })
            .collect();
        DeskSnapshot {
            apps,
            windows,
            spaces: model
                .spaces()
                .iter()
                .map(|space| (space.id.clone(), space.clone()))
                .collect(),
            active: (focus.active.clone(), focus.key),
        }
    }

    /// The effects that bring clients from this state to `after`'s: what appeared or changed
    /// (Spaces, then apps, then windows, so a window finds the objects it names), the active
    /// pair, then what went.
    pub(super) fn changes_to(&self, after: &Model) -> Vec<Effect> {
        let now = DeskSnapshot::of(after);
        let spaces = now
            .spaces
            .iter()
            .filter(|(id, space)| self.spaces.get(*id) != Some(*space))
            .map(|(id, _)| DesktopNote::Space(id.clone()));
        let apps = now
            .apps
            .iter()
            .filter(|(key, desk)| self.apps.get(*key) != Some(*desk))
            .map(|(key, _)| DesktopNote::App(key.clone()));
        let windows = now
            .windows
            .iter()
            .filter(|(id, desk)| self.windows.get(*id) != Some(*desk))
            .map(|(id, _)| DesktopNote::Window(*id));
        let active = (self.active != now.active).then_some(DesktopNote::Active);
        let gone_windows = self
            .windows
            .keys()
            .filter(|id| !now.windows.contains_key(*id))
            .map(|id| DesktopNote::Removed(DesktopRef::Window(*id)));
        let gone_apps = self
            .apps
            .keys()
            .filter(|key| !now.apps.contains_key(*key))
            .map(|key| DesktopNote::Removed(DesktopRef::App(key.clone())));
        let gone_spaces = self
            .spaces
            .keys()
            .filter(|id| !now.spaces.contains_key(*id))
            .map(|id| DesktopNote::Removed(DesktopRef::Space(id.clone())));
        spaces
            .chain(apps)
            .chain(windows)
            .chain(active)
            .chain(gone_windows)
            .chain(gone_apps)
            .chain(gone_spaces)
            .map(Effect::Desktop)
            .collect()
    }
}
