//! What a desktop manager announced, as a client reads it (P2-Q1): the objects with their
//! proxies, the views built from their events, and the `Dispatch` impls that fill them.

use casement_protocols::client::quire_agent_v1::quire_agent_lease_v1::GlowState;
use casement_protocols::client::quire_desktop_v1::quire_desktop_app_v1::{
    self as app, QuireDesktopAppV1,
};
use casement_protocols::client::quire_desktop_v1::quire_desktop_manager_v1::{
    self as manager, ActorKind, QuireDesktopManagerV1,
};
use casement_protocols::client::quire_desktop_v1::quire_desktop_space_v1::{
    self as space, QuireDesktopSpaceV1,
};
use casement_protocols::client::quire_desktop_v1::quire_desktop_window_v1::{
    self as window, QuireDesktopWindowV1,
};
use casement_protocols::client::quire_sensitive_v1::quire_sensitive_surface_v1::SensitivityLevel;
use casement_protocols::client::quire_window_v1::quire_window_v1::{
    Emphasis, PlacementMark, WindowRole,
};
use wayland_client::protocol::wl_output::{self, WlOutput};
use wayland_client::protocol::wl_seat::{self, WlSeat};
use wayland_client::{Connection, Dispatch, QueueHandle, WEnum, event_created_child};
use wayland_protocols::ext::foreign_toplevel_list::v1::client::{
    ext_foreign_toplevel_handle_v1::{self, ExtForeignToplevelHandleV1},
    ext_foreign_toplevel_list_v1::{self, ExtForeignToplevelListV1},
};

/// The value of an enum argument, when the XML names it.
fn known<T>(value: WEnum<T>) -> Option<T> {
    match value {
        WEnum::Value(value) => Some(value),
        WEnum::Unknown(_) => None,
    }
}

/// Whether an object is still there or its last event (`closed`, `removed`) arrived.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum Presence {
    /// No last event yet.
    #[default]
    Here,
    /// The last event arrived.
    Gone,
}

/// An app as announced (a burst applies at `done`; these are the latest values).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct AppView {
    /// The app key (`identity`).
    pub key: String,
    /// How the compositor knows it.
    pub proof: Option<app::ProofKind>,
    /// What the app claims to be.
    pub claimed_id: String,
    /// The program behind it as the compositor established it (`subject`, version 2); `None`
    /// until that event arrives.
    pub subject: Option<String>,
    /// Its life.
    pub life: Option<app::AppLifeKind>,
    /// Whether it is hidden.
    pub visibility: Option<app::AppVisibility>,
    /// How much it asks for the person.
    pub attention: Option<app::Attention>,
    /// Its place in recent use.
    pub mru: Option<u32>,
    /// How many `menu_source` events arrived.
    pub menu_sources: usize,
    /// How many `done` events arrived.
    pub dones: usize,
    /// Whether `closed` arrived.
    pub presence: Presence,
}

/// A window's rectangle and scale as announced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct GeometryView {
    /// Left edge, global logical pixels.
    pub x: i32,
    /// Top edge.
    pub y: i32,
    /// Width.
    pub width: u32,
    /// Height.
    pub height: u32,
    /// The output's scale times 120.
    pub scale120: u32,
}

/// A window as announced.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct WindowView {
    /// The ext-foreign-toplevel identifier.
    pub identifier: String,
    /// The key of its app.
    pub app: Option<String>,
    /// Its role.
    pub role: Option<WindowRole>,
    /// What it blocks.
    pub modality: Option<window::Modality>,
    /// Its state.
    pub state: Option<window::WindowStateKind>,
    /// Who caused the state.
    pub actor: Option<ActorKind>,
    /// Which Spaces show it.
    pub place: Option<window::PlaceKind>,
    /// The id of its Space (for `on_space`).
    pub space: Option<String>,
    /// How many `output` events arrived.
    pub outputs: usize,
    /// Its rectangle.
    pub geometry: Option<GeometryView>,
    /// The identifier of its parent.
    pub parent: Option<String>,
    /// Its emphasis.
    pub emphasis: Option<Emphasis>,
    /// Its sensitivity.
    pub sensitivity: Option<SensitivityLevel>,
    /// An agent's hold on it.
    pub lease: Option<(window::LeaseState, GlowState)>,
    /// Whether its client answers.
    pub responsiveness: Option<window::Responsiveness>,
    /// Where it is placed.
    pub placement: Option<PlacementMark>,
    /// Its menu path.
    pub menu_path: Option<String>,
    /// How many `done` events arrived.
    pub dones: usize,
    /// Whether `closed` arrived.
    pub presence: Presence,
}

/// A Space as announced.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct SpaceView {
    /// Its id (`id`; also ext-workspace's).
    pub id: String,
    /// What it holds.
    pub kind: Option<space::SpaceKind>,
    /// The task token of an agent Space.
    pub occupant: String,
    /// Its place in its output's row.
    pub position: Option<u32>,
    /// How many `output` events arrived.
    pub outputs: usize,
    /// What is happening on it.
    pub activity: Option<space::SpaceActivity>,
    /// How many `done` events arrived.
    pub dones: usize,
    /// Whether `removed` arrived.
    pub presence: Presence,
}

/// The active app and key window as announced.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct ActiveView {
    /// The key of the active app.
    pub app: Option<String>,
    /// The identifier of the key window.
    pub key: Option<String>,
    /// Who caused it.
    pub actor: Option<ActorKind>,
}

/// Where a window object came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Origin {
    /// A `window` event on the manager.
    Announced,
    /// A `get_window_for` request.
    Asked,
}

#[derive(Debug)]
pub(super) struct AppSeen {
    pub(super) object: QuireDesktopAppV1,
    pub(super) view: AppView,
}

#[derive(Debug)]
pub(super) struct WindowSeen {
    pub(super) object: QuireDesktopWindowV1,
    pub(super) origin: Origin,
    pub(super) view: WindowView,
    app: Option<QuireDesktopAppV1>,
    space: Option<QuireDesktopSpaceV1>,
    parent: Option<QuireDesktopWindowV1>,
}

#[derive(Debug)]
pub(super) struct SpaceSeen {
    pub(super) object: QuireDesktopSpaceV1,
    pub(super) view: SpaceView,
}

/// Everything announced on one manager.
#[derive(Debug, Default)]
pub(super) struct Roster {
    pub(super) apps: Vec<AppSeen>,
    pub(super) windows: Vec<WindowSeen>,
    pub(super) spaces: Vec<SpaceSeen>,
    pub(super) handles: Vec<(ExtForeignToplevelHandleV1, String)>,
    active_app: Option<QuireDesktopAppV1>,
    active_key: Option<QuireDesktopWindowV1>,
    active_actor: Option<ActorKind>,
    pub(super) dones: usize,
    pub(super) finished: bool,
}

impl Roster {
    /// A window object that came from `get_window_for`.
    pub(super) fn asked(&mut self, object: QuireDesktopWindowV1) {
        self.windows.push(WindowSeen::new(object, Origin::Asked));
    }

    fn app_key(&self, object: &QuireDesktopAppV1) -> Option<String> {
        let found = self.apps.iter().find(|seen| &seen.object == object)?;
        Some(found.view.key.clone())
    }

    fn space_id(&self, object: &QuireDesktopSpaceV1) -> Option<String> {
        let found = self.spaces.iter().find(|seen| &seen.object == object)?;
        Some(found.view.id.clone())
    }

    fn window_name(&self, object: &QuireDesktopWindowV1) -> Option<String> {
        let found = self.windows.iter().find(|seen| &seen.object == object)?;
        Some(found.view.identifier.clone())
    }

    /// The window views, names resolved, of `origin`.
    pub(super) fn window_views(&self, origin: Origin) -> Vec<WindowView> {
        self.windows
            .iter()
            .filter(|seen| seen.origin == origin)
            .map(|seen| WindowView {
                app: seen.app.as_ref().and_then(|app| self.app_key(app)),
                space: seen.space.as_ref().and_then(|space| self.space_id(space)),
                parent: seen
                    .parent
                    .as_ref()
                    .and_then(|parent| self.window_name(parent)),
                ..seen.view.clone()
            })
            .collect()
    }

    /// The active pair, names resolved.
    pub(super) fn active_view(&self) -> ActiveView {
        ActiveView {
            app: self.active_app.as_ref().and_then(|app| self.app_key(app)),
            key: self
                .active_key
                .as_ref()
                .and_then(|key| self.window_name(key)),
            actor: self.active_actor,
        }
    }
}

impl WindowSeen {
    fn new(object: QuireDesktopWindowV1, origin: Origin) -> WindowSeen {
        WindowSeen {
            object,
            origin,
            view: WindowView::default(),
            app: None,
            space: None,
            parent: None,
        }
    }
}

impl Dispatch<QuireDesktopManagerV1, ()> for Roster {
    fn event(
        state: &mut Roster,
        _proxy: &QuireDesktopManagerV1,
        event: manager::Event,
        _data: &(),
        _connection: &Connection,
        _qh: &QueueHandle<Roster>,
    ) {
        match event {
            manager::Event::App { id } => state.apps.push(AppSeen {
                object: id,
                view: AppView::default(),
            }),
            manager::Event::Window { id } => {
                state.windows.push(WindowSeen::new(id, Origin::Announced));
            }
            manager::Event::Space { id } => state.spaces.push(SpaceSeen {
                object: id,
                view: SpaceView::default(),
            }),
            manager::Event::Active { app, key, actor } => {
                state.active_app = app;
                state.active_key = key;
                state.active_actor = known(actor);
            }
            manager::Event::Done => state.dones += 1,
            manager::Event::Finished => state.finished = true,
            _ => {}
        }
    }

    event_created_child!(Roster, QuireDesktopManagerV1, [
        manager::EVT_APP_OPCODE => (QuireDesktopAppV1, ()),
        manager::EVT_WINDOW_OPCODE => (QuireDesktopWindowV1, ()),
        manager::EVT_SPACE_OPCODE => (QuireDesktopSpaceV1, ()),
    ]);
}

impl Dispatch<QuireDesktopAppV1, ()> for Roster {
    fn event(
        state: &mut Roster,
        proxy: &QuireDesktopAppV1,
        event: app::Event,
        _data: &(),
        _connection: &Connection,
        _qh: &QueueHandle<Roster>,
    ) {
        let Some(seen) = state.apps.iter_mut().find(|seen| &seen.object == proxy) else {
            return;
        };
        let view = &mut seen.view;
        match event {
            app::Event::Identity {
                key,
                proof,
                claimed_id,
            } => {
                view.key = key;
                view.proof = known(proof);
                view.claimed_id = claimed_id;
            }
            app::Event::Subject { subject } => view.subject = Some(subject),
            app::Event::Life { state } => view.life = known(state),
            app::Event::Visibility { state } => view.visibility = known(state),
            app::Event::MenuSource { .. } => view.menu_sources += 1,
            app::Event::Attention { level } => view.attention = known(level),
            app::Event::Mru { rank } => view.mru = Some(rank),
            app::Event::Done => view.dones += 1,
            app::Event::Closed => view.presence = Presence::Gone,
            _ => {}
        }
    }
}

impl Dispatch<QuireDesktopWindowV1, ()> for Roster {
    fn event(
        state: &mut Roster,
        proxy: &QuireDesktopWindowV1,
        event: window::Event,
        _data: &(),
        _connection: &Connection,
        _qh: &QueueHandle<Roster>,
    ) {
        let Some(seen) = state.windows.iter_mut().find(|seen| &seen.object == proxy) else {
            return;
        };
        match event {
            window::Event::Identifier { identifier } => seen.view.identifier = identifier,
            window::Event::App { app } => seen.app = Some(app),
            window::Event::Role { role, modality } => {
                seen.view.role = known(role);
                seen.view.modality = known(modality);
            }
            window::Event::State { kind, actor } => {
                seen.view.state = known(kind);
                seen.view.actor = known(actor);
            }
            window::Event::Place { kind, space } => {
                seen.view.place = known(kind);
                seen.space = space;
            }
            window::Event::Output { .. } => seen.view.outputs += 1,
            window::Event::Geometry {
                x,
                y,
                width,
                height,
                scale120,
            } => {
                seen.view.geometry = Some(GeometryView {
                    x,
                    y,
                    width,
                    height,
                    scale120,
                });
            }
            window::Event::Parent { parent } => seen.parent = parent,
            window::Event::Emphasis { emphasis } => seen.view.emphasis = known(emphasis),
            window::Event::Sensitivity { level } => seen.view.sensitivity = known(level),
            window::Event::Lease { state, glow } => {
                seen.view.lease = known(state).zip(known(glow));
            }
            window::Event::Responsiveness { state } => {
                seen.view.responsiveness = known(state);
            }
            window::Event::Placement { mark } => seen.view.placement = known(mark),
            window::Event::MenuPath { path } => seen.view.menu_path = Some(path),
            window::Event::Done => seen.view.dones += 1,
            window::Event::Closed => seen.view.presence = Presence::Gone,
            _ => {}
        }
    }
}

impl Dispatch<QuireDesktopSpaceV1, ()> for Roster {
    fn event(
        state: &mut Roster,
        proxy: &QuireDesktopSpaceV1,
        event: space::Event,
        _data: &(),
        _connection: &Connection,
        _qh: &QueueHandle<Roster>,
    ) {
        let Some(seen) = state.spaces.iter_mut().find(|seen| &seen.object == proxy) else {
            return;
        };
        let view = &mut seen.view;
        match event {
            space::Event::Id { space_id } => view.id = space_id,
            space::Event::Kind { kind } => view.kind = known(kind),
            space::Event::Occupant { token } => view.occupant = token,
            space::Event::Position { index } => view.position = Some(index),
            space::Event::Output { .. } => view.outputs += 1,
            space::Event::Activity { activity } => view.activity = known(activity),
            space::Event::Done => view.dones += 1,
            space::Event::Removed => view.presence = Presence::Gone,
            _ => {}
        }
    }
}

impl Dispatch<WlOutput, ()> for Roster {
    fn event(
        _state: &mut Roster,
        _proxy: &WlOutput,
        _event: wl_output::Event,
        _data: &(),
        _connection: &Connection,
        _qh: &QueueHandle<Roster>,
    ) {
    }
}

impl Dispatch<WlSeat, ()> for Roster {
    fn event(
        _state: &mut Roster,
        _proxy: &WlSeat,
        _event: wl_seat::Event,
        _data: &(),
        _connection: &Connection,
        _qh: &QueueHandle<Roster>,
    ) {
    }
}

impl Dispatch<ExtForeignToplevelListV1, ()> for Roster {
    fn event(
        state: &mut Roster,
        _proxy: &ExtForeignToplevelListV1,
        event: ext_foreign_toplevel_list_v1::Event,
        _data: &(),
        _connection: &Connection,
        _qh: &QueueHandle<Roster>,
    ) {
        if let ext_foreign_toplevel_list_v1::Event::Toplevel { toplevel } = event {
            state.handles.push((toplevel, String::new()));
        }
    }

    event_created_child!(Roster, ExtForeignToplevelListV1, [
        ext_foreign_toplevel_list_v1::EVT_TOPLEVEL_OPCODE => (ExtForeignToplevelHandleV1, ()),
    ]);
}

impl Dispatch<ExtForeignToplevelHandleV1, ()> for Roster {
    fn event(
        state: &mut Roster,
        proxy: &ExtForeignToplevelHandleV1,
        event: ext_foreign_toplevel_handle_v1::Event,
        _data: &(),
        _connection: &Connection,
        _qh: &QueueHandle<Roster>,
    ) {
        if let ext_foreign_toplevel_handle_v1::Event::Identifier { identifier } = event
            && let Some(held) = state.handles.iter_mut().find(|(handle, _)| handle == proxy)
        {
            held.1 = identifier;
        }
    }
}
