//! What each desktop object tells a manager, as values that compare (P2-Q1): built from the
//! model, kept per manager as what it was last told, and sent as the difference. A burst is
//! the events whose value differs from what was told (all of them for a new object), so
//! publishing the same state twice sends nothing the second time.

use super::DesktopView;
use casement_core::app::{AppKey, AppVisibility as Held, MenuSource};
use casement_core::space::SpaceContent;
use casement_core::window::{Parent, SpacePlace};
use casement_core::{OutputKey, WindowId};
use casement_protocols::server::quire_agent_v1::quire_agent_lease_v1::GlowState;
use casement_protocols::server::quire_desktop_v1::quire_desktop_app_v1::{
    self as app, QuireDesktopAppV1,
};
use casement_protocols::server::quire_desktop_v1::quire_desktop_manager_v1::ActorKind;
use casement_protocols::server::quire_desktop_v1::quire_desktop_space_v1::{
    self as space, QuireDesktopSpaceV1,
};
use casement_protocols::server::quire_desktop_v1::quire_desktop_window_v1::{
    self as window, QuireDesktopWindowV1,
};
use casement_protocols::server::quire_sensitive_v1::quire_sensitive_surface_v1::SensitivityLevel;
use casement_protocols::server::quire_window_v1::quire_window_v1::{
    Emphasis, PlacementMark, WindowRole,
};
use casement_protocols::vocab::{self, WireCode};
use porter_core::SpaceId;
use smithay::reexports::wayland_server::Resource;
use smithay::reexports::wayland_server::protocol::wl_output::WlOutput;
use std::collections::BTreeMap;

/// The wire enum that carries the same code as `value` (`None` for a code the XML lacks).
fn wire<W: TryFrom<u32>>(value: impl WireCode) -> Option<W> {
    W::try_from(value.code()).ok()
}

/// The `wl_output` `object`'s client holds for `key`, when it bound one.
fn output_for(view: &DesktopView<'_>, key: &OutputKey, object: &impl Resource) -> Option<WlOutput> {
    let client = object.client()?;
    view.outputs.get(key)?.client_outputs(&client).next()
}

/// The objects a manager has, for the events that name one object from another.
#[derive(Debug, Clone, Copy)]
pub(super) struct Objects<'a> {
    pub(super) apps: &'a BTreeMap<AppKey, QuireDesktopAppV1>,
    pub(super) spaces: &'a BTreeMap<SpaceId, QuireDesktopSpaceV1>,
    pub(super) windows: &'a BTreeMap<WindowId, QuireDesktopWindowV1>,
}

/// Where an app's global menu is, as `menu_source` carries it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct MenuFacts {
    kind: app::MenuSourceKind,
    bus: String,
    app_path: String,
    menubar_path: String,
}

impl MenuFacts {
    /// `source` as the wire carries it (`None` for a kind the XML lacks).
    fn of(source: &MenuSource) -> Option<MenuFacts> {
        Some(MenuFacts {
            kind: wire(source.kind())?,
            bus: source.bus().to_owned(),
            app_path: source.app_path().to_owned(),
            menubar_path: source.menubar_path().to_owned(),
        })
    }
}

/// An app as a manager is told it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct AppFacts {
    key: String,
    proof: app::ProofKind,
    claimed: String,
    life: app::AppLifeKind,
    visibility: app::AppVisibility,
    attention: app::Attention,
    rank: u32,
    menu: MenuFacts,
}

impl AppFacts {
    /// The app `key` now (`None` when the model has no such app).
    pub(super) fn of(view: &DesktopView<'_>, key: &AppKey) -> Option<AppFacts> {
        let held = view.model.apps().get(key)?;
        let mru = &view.model.focus().app_mru;
        let rank = mru.iter().position(|app| app == key).unwrap_or(mru.len());
        let claimed = held
            .windows
            .iter()
            .find_map(|id| view.model.windows().get(id))
            .map(|window| window.claimed_id.as_str().to_owned())
            .unwrap_or_default();
        let visibility = match held.visibility {
            Held::Shown => vocab::AppVisibility::Shown,
            Held::Hidden { .. } => vocab::AppVisibility::Hidden,
        };
        Some(AppFacts {
            key: key.wire(),
            proof: wire(key.proof())?,
            claimed,
            life: wire(held.life.kind())?,
            visibility: wire(visibility)?,
            attention: wire(held.attention)?,
            rank: u32::try_from(rank).unwrap_or(u32::MAX),
            menu: MenuFacts::of(&view.model.app_menu(key))?,
        })
    }

    /// Sends `object` what differs from `was`; says how many events that was.
    pub(super) fn send(&self, was: Option<&AppFacts>, object: &QuireDesktopAppV1) -> usize {
        let mut sent = 0;
        if was.is_none_or(|was| {
            (&was.key, was.proof, &was.claimed) != (&self.key, self.proof, &self.claimed)
        }) {
            object.identity(self.key.clone(), self.proof, self.claimed.clone());
            sent += 1;
        }
        if was.is_none_or(|was| was.life != self.life) {
            object.life(self.life);
            sent += 1;
        }
        if was.is_none_or(|was| was.visibility != self.visibility) {
            object.visibility(self.visibility);
            sent += 1;
        }
        if was.is_none_or(|was| was.menu != self.menu) {
            let menu = &self.menu;
            object.menu_source(
                menu.kind,
                menu.bus.clone(),
                menu.app_path.clone(),
                menu.menubar_path.clone(),
            );
            sent += 1;
        }
        if was.is_none_or(|was| was.attention != self.attention) {
            object.attention(self.attention);
            sent += 1;
        }
        if was.is_none_or(|was| was.rank != self.rank) {
            object.mru(self.rank);
            sent += 1;
        }
        sent
    }
}

/// A Space as a manager is told it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SpaceFacts {
    id: String,
    kind: space::SpaceKind,
    occupant: String,
    position: u32,
    output: OutputKey,
}

impl SpaceFacts {
    /// The Space `id` now (`None` when the model has no such Space).
    pub(super) fn of(view: &DesktopView<'_>, id: &SpaceId) -> Option<SpaceFacts> {
        let held = view.model.spaces().get(id)?;
        let occupant = match &held.content {
            SpaceContent::Agent(task) => task.0.clone(),
            SpaceContent::Desktop | SpaceContent::FullScreen(_) | SpaceContent::SplitView(_) => {
                String::new()
            }
        };
        Some(SpaceFacts {
            id: held.id.as_str().to_owned(),
            kind: wire(held.content.kind())?,
            occupant,
            position: held.position.0,
            output: held.output.clone(),
        })
    }

    /// Sends `object` what differs from `was`; says how many events that was.
    pub(super) fn send(
        &self,
        was: Option<&SpaceFacts>,
        object: &QuireDesktopSpaceV1,
        view: &DesktopView<'_>,
    ) -> usize {
        let mut sent = 0;
        if was.is_none() {
            object.id(self.id.clone());
            sent += 1;
        }
        if was.is_none_or(|was| was.kind != self.kind) {
            object.kind(self.kind);
            sent += 1;
        }
        if was.is_none_or(|was| was.occupant != self.occupant) {
            object.occupant(self.occupant.clone());
            sent += 1;
        }
        if was.is_none_or(|was| was.position != self.position) {
            object.position(self.position);
            sent += 1;
        }
        if was.is_none_or(|was| was.output != self.output)
            && let Some(output) = output_for(view, &self.output, object)
        {
            object.output(&output);
            sent += 1;
        }
        if was.is_none() {
            object.activity(space::SpaceActivity::None);
            sent += 1;
        }
        sent
    }
}

/// A window's rectangle and the scale of the output it is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Geometry {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    scale120: u32,
}

/// A window as a manager is told it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct WindowFacts {
    identifier: String,
    app: AppKey,
    role: WindowRole,
    modality: window::Modality,
    state: window::WindowStateKind,
    place: window::PlaceKind,
    space: Option<SpaceId>,
    output: Option<OutputKey>,
    geometry: Option<Geometry>,
    parent: Option<WindowId>,
    emphasis: Emphasis,
    sensitivity: SensitivityLevel,
    responsiveness: window::Responsiveness,
    mark: PlacementMark,
    menu_path: String,
}

impl WindowFacts {
    /// The window `id` now (`None` when the model has no such window).
    pub(super) fn of(view: &DesktopView<'_>, id: WindowId) -> Option<WindowFacts> {
        let held = view.model.windows().get(&id)?;
        let output = view.model.output_of(held);
        let scale120 = output
            .as_ref()
            .and_then(|key| view.model.outputs().0.get(key))
            .map_or(120, |plugged| plugged.config.scale.numerator());
        let geometry = view.rects.get(&id).map(|rect| Geometry {
            x: rect.origin.x,
            y: rect.origin.y,
            width: rect.size.w,
            height: rect.size.h,
            scale120,
        });
        let (place, space) = match &held.place {
            SpacePlace::On(on) => (window::PlaceKind::OnSpace, Some(on.clone())),
            SpacePlace::AllSpaces => (window::PlaceKind::AllSpaces, None),
        };
        let parent = match held.parent {
            Parent::None => None,
            Parent::Window(parent) => Some(parent),
        };
        let silent = view
            .model
            .apps()
            .get(&held.app)
            .is_some_and(|app| app.life.kind() == vocab::AppLifeKind::NotResponding);
        let responsiveness = match silent {
            true => window::Responsiveness::NotResponding,
            false => window::Responsiveness::Responding,
        };
        Some(WindowFacts {
            identifier: crate::foreign_toplevel_list::identifier_of(id),
            app: held.app.clone(),
            role: wire(held.role)?,
            modality: wire(held.modality)?,
            state: wire(held.state.kind())?,
            place,
            space,
            output,
            geometry,
            parent,
            emphasis: wire(held.emphasis)?,
            sensitivity: wire(held.sensitivity)?,
            responsiveness,
            mark: wire(held.state.mark())?,
            menu_path: held.hints.menu.window_path().to_owned(),
        })
    }

    /// The app this window belongs to.
    pub(super) fn app(&self) -> &AppKey {
        &self.app
    }

    /// The Space this window is on, when it is on one.
    pub(super) fn space(&self) -> Option<&SpaceId> {
        self.space.as_ref()
    }

    /// Sends `object` what differs from `was`; says how many events that was.
    pub(super) fn send(
        &self,
        was: Option<&WindowFacts>,
        object: &QuireDesktopWindowV1,
        near: Objects<'_>,
        view: &DesktopView<'_>,
    ) -> usize {
        let mut sent = 0;
        if was.is_none() {
            object.identifier(self.identifier.clone());
            sent += 1;
        }
        if was.is_none_or(|was| was.app != self.app)
            && let Some(owner) = near.apps.get(&self.app)
        {
            object.app(owner);
            sent += 1;
        }
        if was.is_none_or(|was| (was.role, was.modality) != (self.role, self.modality)) {
            object.role(self.role, self.modality);
            sent += 1;
        }
        if was.is_none_or(|was| was.state != self.state) {
            object.state(self.state, ActorKind::Unknown);
            sent += 1;
        }
        if was.is_none_or(|was| (was.place, &was.space) != (self.place, &self.space)) {
            let on = self.space.as_ref().and_then(|id| near.spaces.get(id));
            object.place(self.place, on);
            sent += 1;
        }
        if was.is_none_or(|was| was.output != self.output)
            && let Some(key) = &self.output
            && let Some(output) = output_for(view, key, object)
        {
            object.output(&output);
            sent += 1;
        }
        if was.is_none_or(|was| was.geometry != self.geometry)
            && let Some(rect) = self.geometry
        {
            object.geometry(rect.x, rect.y, rect.width, rect.height, rect.scale120);
            sent += 1;
        }
        if was.is_none_or(|was| was.parent != self.parent) {
            let parent = self.parent.and_then(|id| near.windows.get(&id));
            object.parent(parent);
            sent += 1;
        }
        if was.is_none_or(|was| was.emphasis != self.emphasis) {
            object.emphasis(self.emphasis);
            sent += 1;
        }
        if was.is_none_or(|was| was.sensitivity != self.sensitivity) {
            object.sensitivity(self.sensitivity);
            sent += 1;
        }
        if was.is_none() {
            object.lease(window::LeaseState::Free, GlowState::Off);
            sent += 1;
        }
        if was.is_none_or(|was| was.responsiveness != self.responsiveness) {
            object.responsiveness(self.responsiveness);
            sent += 1;
        }
        if was.is_none_or(|was| was.mark != self.mark) {
            object.placement(self.mark);
            sent += 1;
        }
        if was.is_none_or(|was| was.menu_path != self.menu_path) {
            object.menu_path(self.menu_path.clone());
            sent += 1;
        }
        sent
    }
}

/// The active app and key window as a manager is told them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ActiveFacts {
    app: Option<AppKey>,
    key: Option<WindowId>,
}

impl ActiveFacts {
    /// The active pair now.
    pub(super) fn of(view: &DesktopView<'_>) -> ActiveFacts {
        let focus = view.model.focus();
        ActiveFacts {
            app: focus.active.clone(),
            key: focus.key,
        }
    }

    /// What `active` names, resolved to the manager's objects.
    pub(super) fn objects<'a>(
        &self,
        near: Objects<'a>,
    ) -> (
        Option<&'a QuireDesktopAppV1>,
        Option<&'a QuireDesktopWindowV1>,
    ) {
        (
            self.app.as_ref().and_then(|key| near.apps.get(key)),
            self.key.and_then(|id| near.windows.get(&id)),
        )
    }
}
