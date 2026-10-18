//! `quire_desktop_manager_v1` and the objects it makes (`quire_desktop_app_v1`,
//! `quire_desktop_window_v1`, `quire_desktop_space_v1`): the compositor's model of apps,
//! windows and Spaces, published to the shell (which also manages through it) and to the
//! agent tier (which only reads it). Frozen at P2-Z; filled by P2-Q1.
//!
//! The wire glue (the part, the handler trait, the global's and the four objects' dispatch,
//! the refusal of management on an agent connection, the makers that create the objects an
//! event announces) was frozen at P2-Z. What each request means (`on_*`) and the publishing of
//! the model's changes (`apply_note`, the bind burst) are P2-Q1's: `facts` is what an object
//! tells a manager, as values that compare, and `bound` is one manager's objects and what each
//! was last told.

mod bound;
mod facts;

use self::bound::{Asked, Outcome, Told};
use super::code_of;
use crate::client::ClientState;
use crate::foreign_toplevel_list::window_of_identifier;
use crate::handler::EventSink;
use crate::states::{GlobalFilter, WaylandFault};
use casement_core::app::AppKey;
use casement_core::desktop::{AppAsk, DesktopNote, DesktopRequest, SpaceAsk, WindowAsk};
use casement_core::geometry::LogicalRect;
use casement_core::policy::{ConnectionRole, GlobalKind};
use casement_core::space::SpaceIndex;
use casement_core::{Event, Model, OutputKey, WindowId};
use casement_protocols::server::quire_desktop_v1::{
    quire_desktop_app_v1::{self, QuireDesktopAppV1},
    quire_desktop_manager_v1::{self, QuireDesktopManagerV1},
    quire_desktop_space_v1::{self, QuireDesktopSpaceV1},
    quire_desktop_window_v1::{self, QuireDesktopWindowV1},
};
use casement_protocols::vocab::{Assign, PlaceTarget, Toggle, WireCode};
use porter_core::SpaceId;
use smithay::output::Output;
use smithay::reexports::wayland_server::backend::{ClientId, GlobalId};
use smithay::reexports::wayland_server::{
    Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch, New, Resource,
};
use smithay::wayland::foreign_toplevel_list::ForeignToplevelHandle;
use smithay::wayland::{Dispatch2, GlobalDispatch2};
use std::collections::BTreeMap;

/// The version casement serves. Version 2 adds `quire_desktop_app_v1.subject`; the app objects
/// take the manager's version, so only a client that bound the manager at 2 is sent it.
pub const VERSION: u32 = 2;

/// The first version that sends `quire_desktop_app_v1.subject`.
pub const SUBJECT_SINCE: u32 = 2;

/// The version of the window and Space objects: their interfaces have not grown, so they are
/// created at 1 whatever the manager's version is.
const OBJECT_VERSION: u32 = 1;

/// What a connection may do through the desktop manager.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Standing {
    /// The shell: read the model and manage it.
    Manager,
    /// The agent tier: read only; every management request is `not_permitted`.
    ReadOnly,
}

impl Standing {
    /// The standing a connection of `role` has: the shell manages, everything else reads.
    pub fn of(role: &ConnectionRole) -> Standing {
        match role {
            ConnectionRole::Shell => Standing::Manager,
            _ => Standing::ReadOnly,
        }
    }
}

/// One bound manager and the objects it was given.
#[derive(Debug)]
pub struct Bound {
    /// The manager.
    pub manager: QuireDesktopManagerV1,
    /// What its connection may do.
    pub standing: Standing,
    /// Its app objects.
    pub apps: BTreeMap<AppKey, QuireDesktopAppV1>,
    /// Its window objects.
    pub windows: BTreeMap<WindowId, QuireDesktopWindowV1>,
    /// Its Space objects.
    pub spaces: BTreeMap<SpaceId, QuireDesktopSpaceV1>,
    /// Whether the client sent `stop` (no more `app`, `window` or `space` events).
    pub stopped: Stopped,
    /// The window objects the client made with `get_window_for`, beside the one the bind gave.
    pub asked: Vec<Asked>,
    /// What each object was last told.
    pub told: Told,
}

/// Whether a manager was stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Stopped {
    /// It still gets new objects.
    No,
    /// It asked for no more and was sent `finished`.
    Yes,
}

/// `quire_desktop_manager_v1` and everything bound on it.
#[derive(Debug)]
pub struct QuireDesktopPart {
    /// The global.
    pub global: GlobalId,
    /// Every bound manager.
    pub bound: Vec<Bound>,
    /// How to create the objects an event announces.
    pub makers: Makers,
}

impl QuireDesktopPart {
    /// The part; creates the global (at [`VERSION`]) behind the I7 filter.
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> QuireDesktopPart
    where
        D: GlobalDispatch<QuireDesktopManagerV1, DesktopGlobal>
            + Dispatch<QuireDesktopAppV1, AppData>
            + Dispatch<QuireDesktopWindowV1, WindowData>
            + Dispatch<QuireDesktopSpaceV1, SpaceData>
            + 'static,
    {
        let global = display.create_global::<D, QuireDesktopManagerV1, _>(
            VERSION,
            DesktopGlobal {
                filter: role_filter,
            },
        );
        QuireDesktopPart {
            global,
            bound: Vec::new(),
            makers: Makers::new::<D>(display),
        }
    }
}

/// How to create the app, window and Space objects a manager announces (`app`, `window` and
/// `space` events carry a `new_id`). Built where the state type is known, so the publishing
/// functions need no type parameter.
#[derive(Debug, Clone)]
pub struct Makers {
    display: DisplayHandle,
    app: fn(&DisplayHandle, &QuireDesktopManagerV1, AppData) -> Option<QuireDesktopAppV1>,
    window: fn(&DisplayHandle, &QuireDesktopManagerV1, WindowData) -> Option<QuireDesktopWindowV1>,
    space: fn(&DisplayHandle, &QuireDesktopManagerV1, SpaceData) -> Option<QuireDesktopSpaceV1>,
}

impl Makers {
    fn new<D>(display: &DisplayHandle) -> Makers
    where
        D: Dispatch<QuireDesktopAppV1, AppData>
            + Dispatch<QuireDesktopWindowV1, WindowData>
            + Dispatch<QuireDesktopSpaceV1, SpaceData>
            + 'static,
    {
        Makers {
            display: display.clone(),
            app: |display, manager, data| {
                let client = manager.client()?;
                let handle = client
                    .create_resource::<QuireDesktopAppV1, AppData, D>(
                        display,
                        manager.version(),
                        data,
                    )
                    .ok()?;
                manager.app(&handle);
                Some(handle)
            },
            window: |display, manager, data| {
                let client = manager.client()?;
                let handle = client
                    .create_resource::<QuireDesktopWindowV1, WindowData, D>(
                        display,
                        manager.version().min(OBJECT_VERSION),
                        data,
                    )
                    .ok()?;
                manager.window(&handle);
                Some(handle)
            },
            space: |display, manager, data| {
                let client = manager.client()?;
                let handle = client
                    .create_resource::<QuireDesktopSpaceV1, SpaceData, D>(
                        display,
                        manager.version().min(OBJECT_VERSION),
                        data,
                    )
                    .ok()?;
                manager.space(&handle);
                Some(handle)
            },
        }
    }

    /// Creates an app object on `manager` and announces it (`app`); its state burst is the
    /// caller's. `None` when the client is gone.
    pub fn make_app(
        &self,
        manager: &QuireDesktopManagerV1,
        data: AppData,
    ) -> Option<QuireDesktopAppV1> {
        (self.app)(&self.display, manager, data)
    }

    /// Creates a window object on `manager` and announces it (`window`).
    pub fn make_window(
        &self,
        manager: &QuireDesktopManagerV1,
        data: WindowData,
    ) -> Option<QuireDesktopWindowV1> {
        (self.window)(&self.display, manager, data)
    }

    /// Creates a Space object on `manager` and announces it (`space`).
    pub fn make_space(
        &self,
        manager: &QuireDesktopManagerV1,
        data: SpaceData,
    ) -> Option<QuireDesktopSpaceV1> {
        (self.space)(&self.display, manager, data)
    }
}

/// What the publishing functions read of the compositor: the model, the `wl_output`s and
/// where the last arrangement put each window.
#[derive(Debug, Clone)]
pub struct DesktopView<'a> {
    /// The model.
    pub model: &'a Model,
    /// The `wl_output` globals of the plugged displays, by key.
    pub outputs: &'a BTreeMap<OutputKey, Output>,
    /// Each arranged window's rectangle in global logical pixels (a minimised window has
    /// none); `geometry` events come from here.
    pub rects: BTreeMap<WindowId, LogicalRect>,
}

/// The part and what it reads, borrowed together (they sit in different fields of the state).
#[derive(Debug)]
pub struct DesktopParts<'a> {
    /// The part.
    pub part: &'a mut QuireDesktopPart,
    /// What it reads.
    pub view: DesktopView<'a>,
}

/// What the composition root provides to this protocol's dispatch.
pub trait QuireDesktopHandler: EventSink + 'static {
    /// The part and the model it reads.
    fn quire_desktop_parts(&mut self) -> DesktopParts<'_>;
}

/// The global's data: the filter its `can_view` asks.
#[derive(Debug, Clone, Copy)]
pub struct DesktopGlobal {
    filter: GlobalFilter,
}

/// A bound manager's data.
#[derive(Debug, Clone, Copy)]
pub struct ManagerData {
    /// What its connection may do.
    pub standing: Standing,
}

/// An app object's data.
#[derive(Debug, Clone)]
pub struct AppData {
    /// The app.
    pub app: AppKey,
    /// What the connection may do.
    pub standing: Standing,
    /// The manager it came from, which a refused request is raised on.
    pub manager: QuireDesktopManagerV1,
}

/// A window object's data.
#[derive(Debug, Clone)]
pub struct WindowData {
    /// The window, when the toplevel handle named a live one.
    pub window: Option<WindowId>,
    /// What the connection may do.
    pub standing: Standing,
    /// The manager it came from, which a refused request is raised on.
    pub manager: QuireDesktopManagerV1,
}

/// A Space object's data.
#[derive(Debug, Clone)]
pub struct SpaceData {
    /// The Space.
    pub space: SpaceId,
    /// What the connection may do.
    pub standing: Standing,
    /// The manager it came from, which a refused request is raised on.
    pub manager: QuireDesktopManagerV1,
}

impl<D> GlobalDispatch2<QuireDesktopManagerV1, D> for DesktopGlobal
where
    D: QuireDesktopHandler + Dispatch<QuireDesktopManagerV1, ManagerData> + 'static,
{
    fn bind(
        &self,
        state: &mut D,
        _handle: &DisplayHandle,
        client: &Client,
        resource: New<QuireDesktopManagerV1>,
        data_init: &mut DataInit<'_, D>,
    ) {
        let role = ClientState::of(client).map_or(ConnectionRole::Public, |data| data.role.clone());
        let standing = Standing::of(&role);
        let manager = data_init.init(resource, ManagerData { standing });
        let DesktopParts { part, view } = state.quire_desktop_parts();
        if let Err(fault) = on_bind(part, &view, manager, standing) {
            tracing::warn!(%fault, "a desktop manager could not be given its state");
        }
    }

    fn can_view(&self, client: &Client) -> bool {
        self.filter.sees(GlobalKind::QuireDesktop, client)
    }
}

/// Posts the manager's `not_permitted` (an agent connection made a management request) and
/// says whether the request was refused. The error is the manager's, so it goes on the
/// manager, whichever of its objects asked.
fn refused(standing: Standing, manager: &QuireDesktopManagerV1) -> bool {
    match standing {
        Standing::Manager => false,
        Standing::ReadOnly => {
            manager.post_error(
                quire_desktop_manager_v1::Error::NotPermitted,
                "management requests are not permitted on this connection",
            );
            true
        }
    }
}

impl<D> Dispatch2<QuireDesktopManagerV1, D> for ManagerData
where
    D: QuireDesktopHandler + Dispatch<QuireDesktopWindowV1, WindowData>,
{
    fn request(
        &self,
        state: &mut D,
        _client: &Client,
        resource: &QuireDesktopManagerV1,
        request: quire_desktop_manager_v1::Request,
        _dhandle: &DisplayHandle,
        data_init: &mut DataInit<'_, D>,
    ) {
        match request {
            quire_desktop_manager_v1::Request::GetWindowFor { id, toplevel } => {
                let window = ForeignToplevelHandle::from_resource(&toplevel)
                    .and_then(|handle| window_of_identifier(&handle.identifier()));
                let object = data_init.init(
                    id,
                    WindowData {
                        window,
                        standing: self.standing,
                        manager: resource.clone(),
                    },
                );
                let Some(window) = window else {
                    return;
                };
                let DesktopParts { part, view } = state.quire_desktop_parts();
                if let Err(fault) = on_get_window_for(part, &view, resource, window, object) {
                    tracing::warn!(%fault, "a desktop window could not be given its state");
                }
            }
            quire_desktop_manager_v1::Request::CreateSpace { output } => {
                if refused(self.standing, resource) {
                    return;
                }
                let DesktopParts { part, view } = state.quire_desktop_parts();
                let key = Output::from_resource(&output).and_then(|asked| {
                    view.outputs
                        .iter()
                        .find(|(_, shown)| **shown == asked)
                        .map(|(key, _)| key.clone())
                });
                let Some(key) = key else {
                    resource.post_error(
                        quire_desktop_manager_v1::Error::UnknownOutput,
                        "the wl_output is not a live output",
                    );
                    return;
                };
                let events = on_create_space(part, key);
                state.feed(events);
            }
            quire_desktop_manager_v1::Request::Stop => {
                let DesktopParts { part, .. } = state.quire_desktop_parts();
                on_stop(part, resource);
            }
            _ => {}
        }
    }

    fn destroyed(&self, state: &mut D, _client: ClientId, resource: &QuireDesktopManagerV1) {
        on_manager_gone(state.quire_desktop_parts().part, resource);
    }
}

impl<D> Dispatch2<QuireDesktopAppV1, D> for AppData
where
    D: QuireDesktopHandler,
{
    fn request(
        &self,
        state: &mut D,
        _client: &Client,
        _resource: &QuireDesktopAppV1,
        request: quire_desktop_app_v1::Request,
        _dhandle: &DisplayHandle,
        _data_init: &mut DataInit<'_, D>,
    ) {
        let ask = match request {
            // As for a window: the shell's serial (0 from `desktop-client`) is accepted unread.
            quire_desktop_app_v1::Request::Activate { .. } => AppAsk::Activate,
            quire_desktop_app_v1::Request::Hide => AppAsk::Hide,
            quire_desktop_app_v1::Request::Unhide => AppAsk::Unhide,
            quire_desktop_app_v1::Request::HideOthers => AppAsk::HideOthers,
            quire_desktop_app_v1::Request::Quit => AppAsk::Quit,
            quire_desktop_app_v1::Request::ForceQuit => AppAsk::ForceQuit,
            quire_desktop_app_v1::Request::Reopen => AppAsk::Reopen,
            _ => return,
        };
        if refused(self.standing, &self.manager) {
            return;
        }
        let events = on_app(state.quire_desktop_parts().part, &self.app, ask);
        state.feed(events);
    }
}

impl<D> Dispatch2<QuireDesktopWindowV1, D> for WindowData
where
    D: QuireDesktopHandler,
{
    fn request(
        &self,
        state: &mut D,
        _client: &Client,
        _resource: &QuireDesktopWindowV1,
        request: quire_desktop_window_v1::Request,
        _dhandle: &DisplayHandle,
        _data_init: &mut DataInit<'_, D>,
    ) {
        let ask = match request {
            // The input serial is not checked: the connection that may manage is the shell's,
            // and the shell's activation is the person's hand (a dock or switcher click), so a
            // serial of 0 from it is accepted; an agent connection is refused below.
            quire_desktop_window_v1::Request::Activate { .. } => Some(WindowAsk::Activate),
            quire_desktop_window_v1::Request::Close => Some(WindowAsk::Close),
            quire_desktop_window_v1::Request::Minimise => Some(WindowAsk::Minimise),
            quire_desktop_window_v1::Request::Unminimise => Some(WindowAsk::Unminimise),
            quire_desktop_window_v1::Request::Place { target } => code_of(target)
                .and_then(|code| PlaceTarget::from_code(code).ok())
                .map(WindowAsk::Place),
            quire_desktop_window_v1::Request::FullScreen { toggle } => code_of(toggle)
                .and_then(|code| Toggle::from_code(code).ok())
                .map(WindowAsk::FullScreen),
            quire_desktop_window_v1::Request::ToSpace { space } => space
                .data::<SpaceData>()
                .map(|data| WindowAsk::ToSpace(data.space.clone())),
            quire_desktop_window_v1::Request::Assign { assign } => code_of(assign)
                .and_then(|code| Assign::from_code(code).ok())
                .map(WindowAsk::Assign),
            _ => None,
        };
        let (Some(ask), Some(window)) = (ask, self.window) else {
            return;
        };
        if refused(self.standing, &self.manager) {
            return;
        }
        let events = on_window(state.quire_desktop_parts().part, window, ask);
        state.feed(events);
    }
}

impl<D> Dispatch2<QuireDesktopSpaceV1, D> for SpaceData
where
    D: QuireDesktopHandler,
{
    fn request(
        &self,
        state: &mut D,
        _client: &Client,
        _resource: &QuireDesktopSpaceV1,
        request: quire_desktop_space_v1::Request,
        _dhandle: &DisplayHandle,
        _data_init: &mut DataInit<'_, D>,
    ) {
        let ask = match request {
            quire_desktop_space_v1::Request::Activate => SpaceAsk::Activate,
            quire_desktop_space_v1::Request::Remove => SpaceAsk::Remove,
            quire_desktop_space_v1::Request::MoveTo { position } => {
                SpaceAsk::MoveTo(SpaceIndex(position))
            }
            _ => return,
        };
        if refused(self.standing, &self.manager) {
            return;
        }
        let events = on_space(state.quire_desktop_parts().part, &self.space, ask);
        state.feed(events);
    }
}

/// A manager was bound: record it and send it the whole desktop. The order is the Spaces and
/// the apps (each `space` or `app` event with its state burst), then a `window` event for
/// every window, so a window's burst can name any app, Space or parent, then each window's
/// burst, `active` and `done`.
pub fn on_bind(
    part: &mut QuireDesktopPart,
    view: &DesktopView<'_>,
    manager: QuireDesktopManagerV1,
    standing: Standing,
) -> Result<(), WaylandFault> {
    let QuireDesktopPart { bound, makers, .. } = part;
    let mut fresh = Bound::new(manager, standing);
    for space in view.model.spaces().iter() {
        fresh.space(makers, view, &space.id);
    }
    for key in view.model.apps().keys() {
        fresh.app(makers, view, key);
    }
    let windows: Vec<WindowId> = view.model.windows().keys().copied().collect();
    for id in &windows {
        fresh.announce_window(makers, *id);
    }
    for id in windows {
        fresh.window(makers, view, id);
    }
    fresh.active(view);
    fresh.manager.done();
    bound.push(fresh);
    Ok(())
}

/// `get_window_for`: `object` is the desktop window of `window`; record it and send its
/// state burst (it is closed at once when the model has no such window).
pub fn on_get_window_for(
    part: &mut QuireDesktopPart,
    view: &DesktopView<'_>,
    manager: &QuireDesktopManagerV1,
    window: WindowId,
    object: QuireDesktopWindowV1,
) -> Result<(), WaylandFault> {
    let QuireDesktopPart { bound, makers, .. } = part;
    let Some(held) = bound.iter_mut().find(|held| held.manager == *manager) else {
        return Ok(());
    };
    held.prune();
    if held
        .ask_window(makers, view, window, object)
        .closes_a_burst()
    {
        held.manager.done();
    }
    Ok(())
}

/// `create_space` on `output` (already checked live): the request the model takes.
pub fn on_create_space(_part: &mut QuireDesktopPart, output: OutputKey) -> Vec<Event> {
    vec![Event::Desktop(DesktopRequest::CreateSpace { output })]
}

/// `stop`: send `manager` nothing new and answer `finished` (once).
pub fn on_stop(part: &mut QuireDesktopPart, manager: &QuireDesktopManagerV1) {
    let Some(held) = part.bound.iter_mut().find(|held| held.manager == *manager) else {
        return;
    };
    if held.stopped == Stopped::No {
        held.stopped = Stopped::Yes;
        manager.finished();
    }
}

/// A request on an app (`Event::Desktop(DesktopRequest::App { .. })`).
pub fn on_app(_part: &mut QuireDesktopPart, app: &AppKey, ask: AppAsk) -> Vec<Event> {
    vec![Event::Desktop(DesktopRequest::App {
        app: app.clone(),
        ask,
    })]
}

/// A request on a window (`Event::Desktop(DesktopRequest::Window { .. })`).
pub fn on_window(_part: &mut QuireDesktopPart, window: WindowId, ask: WindowAsk) -> Vec<Event> {
    vec![Event::Desktop(DesktopRequest::Window { window, ask })]
}

/// A request on a Space (`Event::Desktop(DesktopRequest::Space { .. })`).
pub fn on_space(_part: &mut QuireDesktopPart, space: &SpaceId, ask: SpaceAsk) -> Vec<Event> {
    vec![Event::Desktop(DesktopRequest::Space {
        space: space.clone(),
        ask,
    })]
}

/// A manager's object went away (the client destroyed it or disconnected).
pub fn on_manager_gone(part: &mut QuireDesktopPart, manager: &QuireDesktopManagerV1) {
    part.bound.retain(|bound| bound.manager != *manager);
}

/// The model said `note`: send every bound manager the object it names as it now is (a new
/// object is created and announced first), or its `closed` and `removed` end, then `done`.
/// A manager that was stopped gets no new object. Saying the same thing again sends nothing:
/// each object remembers what it was told.
pub fn apply_note(
    part: &mut QuireDesktopPart,
    view: &DesktopView<'_>,
    note: &DesktopNote,
) -> Result<(), WaylandFault> {
    let QuireDesktopPart { bound, makers, .. } = part;
    for held in bound.iter_mut() {
        held.prune();
        let outcome = match note {
            DesktopNote::App(key) => held.app(makers, view, key),
            DesktopNote::Window(id) => held.window(makers, view, *id),
            DesktopNote::Space(id) => held.space(makers, view, id),
            DesktopNote::Active => held.active(view),
            DesktopNote::Removed(gone) => held.removed(gone),
            _ => Outcome::Same,
        };
        if outcome.closes_a_burst() {
            held.manager.done();
        }
    }
    Ok(())
}
