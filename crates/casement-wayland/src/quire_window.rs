//! `quire_window_manager_v1` (C50): what an app says about its toplevel (first-mouse regions,
//! frame, role, accessibility root, edited) and what it learns back (emphasis, placement).
//! Advertised from P0 so shell-host can tell casement apart by this global; requests are
//! stored in the model, `request_placement` is acted on from P2 ([`PLACEMENT_POLICY`]).
//!
//! smithay has no module for our protocol, so this one carries its own `GlobalDispatch2` and
//! `Dispatch2` impls; `delegate_dispatch2!` in the composition root routes to them, and they
//! reach the root through [`QuireWindowHandler`].
//!
//! Version 2 adds the manager's `quit` event: the person asked the app to quit. A client that
//! binds the manager at version 2 or later is reported to the model as a quit listener
//! ([`Event::QuitListener`]) for as long as it holds the manager, and the model then sends it
//! `quit` ([`apply_quit`]) in place of closing its windows.
//!
//! What each request means is in [`hints`] as pure functions; this file is the wire: the part,
//! the handler trait, the two dispatch impls and the three effects.

mod hints;

pub use hints::{
    A11Y_MAX_LEN, A11yTooLong, BadRole, PLACEMENT_POLICY, PendingHints, PlacementPolicy, a11y_of,
    first_mouse_of, forgotten, frame_of, placement_events, role_request,
};

use crate::client::ClientState;
use crate::states::{GlobalFilter, WaylandFault};
use casement_core::geometry::LogicalRect;
use casement_core::policy::GlobalKind;
use casement_core::window::{A11yRoot, FrameHint};
use casement_core::{ClientKey, Event, InputSerial, WindowHintChange, WindowId};
use casement_protocols::server::quire_window_v1::quire_window_manager_v1::{
    self, QuireWindowManagerV1,
};
use casement_protocols::server::quire_window_v1::quire_window_v1::{self, QuireWindowV1};
use casement_protocols::vocab::{
    Edited, Emphasis, FrameMode, PlaceTarget, PlacementMark, Toggle, WindowRole, WireCode,
};
use smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel::XdgToplevel;
use smithay::reexports::wayland_server::backend::{ClientId, GlobalId};
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::{
    Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch, New, Resource, WEnum,
};
use smithay::utils::IsAlive;
use smithay::wayland::compositor::{add_post_commit_hook, get_region_attributes};
use smithay::wayland::{Dispatch2, GlobalDispatch2};
use std::collections::{BTreeMap, BTreeSet};

/// The version casement serves.
pub const VERSION: u32 = 2;

/// The first version that carries `quit`.
pub const QUIT_VERSION: u32 = 2;

/// `quire_window_manager_v1` and its per-toplevel objects.
#[derive(Debug)]
pub struct QuireWindowPart {
    /// The global.
    pub global: GlobalId,
    /// Each extended toplevel's object (at most one per window).
    pub windows: BTreeMap<WindowId, QuireWindowV1>,
    /// The double-buffered values each window staged since its last commit.
    pub pending: BTreeMap<WindowId, PendingHints>,
    /// The last placement each window asked for with a proven press.
    pub placements: BTreeMap<WindowId, (PlaceTarget, InputSerial)>,
    /// The placement mark each extended window was last told, so a state change that leaves the
    /// mark where it was sends nothing.
    pub marks: BTreeMap<WindowId, PlacementMark>,
    /// The windows whose surface carries our commit hook (it is added once).
    hooked: BTreeSet<WindowId>,
    /// The hints each window committed before the model held it, in order: a client may say
    /// them before its first buffer, and the hook runs before the commit maps the window, so
    /// they are fed again just after its `ToplevelMapped`.
    pub early: BTreeMap<WindowId, Vec<Event>>,
    /// Each live manager bound at [`QUIT_VERSION`] or later, with its client: where `quit` goes.
    pub quit_managers: Vec<(ClientKey, QuireWindowManagerV1)>,
}

impl QuireWindowPart {
    /// Creates the global (version 2), filtered by the I7 table.
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> QuireWindowPart
    where
        D: GlobalDispatch<QuireWindowManagerV1, QuireWindowGlobal> + 'static,
    {
        QuireWindowPart {
            global: display.create_global::<D, QuireWindowManagerV1, _>(
                VERSION,
                QuireWindowGlobal {
                    filter: role_filter,
                },
            ),
            windows: BTreeMap::new(),
            pending: BTreeMap::new(),
            placements: BTreeMap::new(),
            marks: BTreeMap::new(),
            hooked: BTreeSet::new(),
            early: BTreeMap::new(),
            quit_managers: Vec::new(),
        }
    }
}

/// What the composition root knows of a window the extension reports back.
#[derive(Debug, Clone)]
pub struct WindowFacts {
    /// Key, main or inactive.
    pub emphasis: Emphasis,
    /// Where it sits, as its zoom control shows it.
    pub mark: PlacementMark,
    /// The role it has without this object (from `xdg_dialog` and the shell protocols).
    pub base_role: WindowRole,
    /// Its root surface (the double-buffered values apply at its commits).
    pub surface: WlSurface,
}

/// What the composition root provides to this protocol's dispatch.
pub trait QuireWindowHandler: 'static {
    /// The part.
    fn quire_window_part(&mut self) -> &mut QuireWindowPart;
    /// The window an `xdg_toplevel` is, if it is live.
    fn window_of_toplevel(&self, toplevel: &XdgToplevel) -> Option<WindowId>;
    /// Feeds the events a request produced into the model (`core_loop::feed`).
    fn quire_window_events(&mut self, events: Vec<Event>);
    /// What the model says of `window` now (`None` once it is gone).
    fn window_facts(&self, window: WindowId) -> Option<WindowFacts>;
    /// Whether `serial` answers a recent press the seat delivered to `client`.
    fn press_proves(&self, client: &Client, serial: InputSerial) -> bool;
}

/// The global's data: the filter its `can_view` asks.
#[derive(Debug, Clone, Copy)]
pub struct QuireWindowGlobal {
    filter: GlobalFilter,
}

/// A bound manager's data.
#[derive(Debug, Clone, Copy, Default)]
pub struct QuireWindowManagerData;

/// One extension object's data: the toplevel it extends and the window that is.
#[derive(Debug, Clone)]
pub struct QuireWindowData {
    /// The extended toplevel. Once it is gone the object is defunct.
    pub toplevel: XdgToplevel,
    /// Its window, as it was when the object was created (`None`: no live window then).
    pub window: Option<WindowId>,
}

impl<D> GlobalDispatch2<QuireWindowManagerV1, D> for QuireWindowGlobal
where
    D: QuireWindowHandler + Dispatch<QuireWindowManagerV1, QuireWindowManagerData> + 'static,
{
    fn bind(
        &self,
        state: &mut D,
        _handle: &DisplayHandle,
        client: &Client,
        resource: New<QuireWindowManagerV1>,
        data_init: &mut DataInit<'_, D>,
    ) {
        let manager = data_init.init(resource, QuireWindowManagerData);
        if manager.version() < QUIT_VERSION {
            return;
        }
        let Some(key) = ClientState::of(client).map(|data| data.key) else {
            return;
        };
        let events = on_quit_manager_bound(state.quire_window_part(), key, manager);
        state.quire_window_events(events);
    }

    fn can_view(&self, client: &Client) -> bool {
        self.filter.sees(GlobalKind::QuireWindow, client)
    }
}

impl<D> Dispatch2<QuireWindowManagerV1, D> for QuireWindowManagerData
where
    D: QuireWindowHandler + Dispatch<QuireWindowV1, QuireWindowData>,
{
    fn request(
        &self,
        state: &mut D,
        _client: &Client,
        resource: &QuireWindowManagerV1,
        request: quire_window_manager_v1::Request,
        _dhandle: &DisplayHandle,
        data_init: &mut DataInit<'_, D>,
    ) {
        match request {
            quire_window_manager_v1::Request::GetWindow { id, toplevel } => {
                let window = state.window_of_toplevel(&toplevel);
                // The new id is always initialised (smithay's dispatch requires it), even when
                // the request is an error.
                let object = data_init.init(id, QuireWindowData { toplevel, window });
                let Some(window) = window else {
                    return;
                };
                if state.quire_window_part().windows.contains_key(&window) {
                    resource.post_error(
                        quire_window_manager_v1::Error::AlreadyConstructed,
                        "the toplevel already has a quire_window_v1 object",
                    );
                    return;
                }
                let events = on_get_window(state.quire_window_part(), window, object.clone());
                if let Some(facts) = state.window_facts(window) {
                    send_emphasis(&object, facts.emphasis);
                    send_placement(&object, facts.mark);
                    state.quire_window_part().marks.insert(window, facts.mark);
                    hook_commits(state, window, &facts.surface);
                }
                state.quire_window_events(events);
            }
            quire_window_manager_v1::Request::Destroy => {}
            _ => {}
        }
    }

    fn destroyed(&self, state: &mut D, _client: ClientId, resource: &QuireWindowManagerV1) {
        let events = on_quit_manager_gone(state.quire_window_part(), resource);
        if !events.is_empty() {
            state.quire_window_events(events);
        }
    }
}

/// Applies the staged values at `window`'s commits (the double-buffered requests).
fn hook_commits<D: QuireWindowHandler>(state: &mut D, window: WindowId, surface: &WlSurface) {
    if !state.quire_window_part().hooked.insert(window) {
        return;
    }
    add_post_commit_hook::<D, _>(surface, move |state, _display, _surface| {
        let events = on_commit(state.quire_window_part(), window);
        if !events.is_empty() {
            state.quire_window_events(events);
        }
    });
}

impl<D> Dispatch2<QuireWindowV1, D> for QuireWindowData
where
    D: QuireWindowHandler,
{
    fn request(
        &self,
        state: &mut D,
        client: &Client,
        resource: &QuireWindowV1,
        request: quire_window_v1::Request,
        _dhandle: &DisplayHandle,
        _data_init: &mut DataInit<'_, D>,
    ) {
        if matches!(request, quire_window_v1::Request::Destroy) {
            return;
        }
        let live = self.window.filter(|window| {
            self.toplevel.alive() && state.window_of_toplevel(&self.toplevel) == Some(*window)
        });
        let Some(window) = live else {
            resource.post_error(
                quire_window_v1::Error::DefunctToplevel,
                "the xdg_toplevel is gone",
            );
            return;
        };
        let events = match request {
            quire_window_v1::Request::SetFirstMouse { region } => {
                let first_mouse = region.as_ref().map_or_else(Vec::new, |region| {
                    first_mouse_of(&get_region_attributes(region))
                });
                on_set_first_mouse(state.quire_window_part(), window, first_mouse)
            }
            quire_window_v1::Request::SetFrame {
                mode,
                corner_radius,
            } => match code_of(mode).and_then(|code| FrameMode::from_code(code).ok()) {
                Some(mode) => on_set_frame(
                    state.quire_window_part(),
                    window,
                    frame_of(mode, corner_radius),
                ),
                None => Vec::new(),
            },
            quire_window_v1::Request::RequestPlacement { target, serial } => {
                match code_of(target).and_then(|code| PlaceTarget::from_code(code).ok()) {
                    Some(target) if state.press_proves(client, InputSerial(serial)) => {
                        on_request_placement(
                            state.quire_window_part(),
                            window,
                            target,
                            InputSerial(serial),
                        )
                    }
                    Some(_) | None => Vec::new(),
                }
            }
            quire_window_v1::Request::SetRole { role } => {
                match code_of(role).and_then(|code| WindowRole::from_code(code).ok()) {
                    Some(role) => match role_request(role) {
                        Ok(role) => on_set_role(state.quire_window_part(), window, role),
                        Err(BadRole) => {
                            resource.post_error(
                                quire_window_v1::Error::BadRole,
                                "dialog and shell are not roles a client may ask for",
                            );
                            return;
                        }
                    },
                    None => Vec::new(),
                }
            }
            quire_window_v1::Request::SetA11yRoot {
                bus_name,
                object_path,
            } => match a11y_of(&bus_name, &object_path) {
                Ok(root) => on_set_a11y_root(state.quire_window_part(), window, root),
                Err(A11yTooLong) => Vec::new(),
            },
            quire_window_v1::Request::SetEdited { edited } => {
                match code_of(edited).and_then(|code| Edited::from_code(code).ok()) {
                    Some(edited) => on_set_edited(state.quire_window_part(), window, edited),
                    None => Vec::new(),
                }
            }
            _ => Vec::new(),
        };
        state.quire_window_events(events);
    }

    fn destroyed(&self, state: &mut D, _client: ClientId, resource: &QuireWindowV1) {
        let Some(window) = self.window else {
            return;
        };
        let part = state.quire_window_part();
        if part.windows.get(&window) != Some(resource) {
            return;
        }
        part.windows.remove(&window);
        part.pending.remove(&window);
        part.placements.remove(&window);
        part.marks.remove(&window);
        if !self.toplevel.alive() {
            return;
        }
        if let Some(facts) = state.window_facts(window) {
            let events = forgotten(facts.base_role)
                .into_iter()
                .map(|hint| Event::WindowHint { window, hint })
                .collect();
            state.quire_window_events(events);
        }
    }
}

/// The wire code of a request's enum argument (`None` for a value the XML does not name).
fn code_of<T: Into<u32>>(value: WEnum<T>) -> Option<u32> {
    match value {
        WEnum::Value(value) => Some(value.into()),
        WEnum::Unknown(_) => None,
    }
}

fn send_emphasis(object: &QuireWindowV1, emphasis: Emphasis) {
    if let Some(wire) = emphasis.to_wire() {
        object.emphasis(wire);
    }
}

fn send_placement(object: &QuireWindowV1, mark: PlacementMark) {
    if let Some(wire) = mark.to_wire() {
        object.placement(wire);
    }
}

/// A manager bound at version 2 or later: its client now hears `quit`. The model is told when
/// it is the client's first such manager.
pub fn on_quit_manager_bound(
    part: &mut QuireWindowPart,
    client: ClientKey,
    manager: QuireWindowManagerV1,
) -> Vec<Event> {
    let first = !part.quit_managers.iter().any(|(held, _)| *held == client);
    part.quit_managers.push((client, manager));
    match first {
        true => vec![Event::QuitListener {
            client,
            toggle: Toggle::On,
        }],
        false => Vec::new(),
    }
}

/// A manager was destroyed (by its client, or with the connection). The model is told when it
/// was the last one its client held at version 2 or later.
pub fn on_quit_manager_gone(
    part: &mut QuireWindowPart,
    manager: &QuireWindowManagerV1,
) -> Vec<Event> {
    let Some(at) = part
        .quit_managers
        .iter()
        .position(|(_, held)| held == manager)
    else {
        return Vec::new();
    };
    let (client, _) = part.quit_managers.remove(at);
    match part.quit_managers.iter().any(|(held, _)| *held == client) {
        true => Vec::new(),
        false => vec![Event::QuitListener {
            client,
            toggle: Toggle::Off,
        }],
    }
}

/// `get_window`: `window`'s toplevel was extended with `object`.
pub fn on_get_window(
    part: &mut QuireWindowPart,
    window: WindowId,
    object: QuireWindowV1,
) -> Vec<Event> {
    part.windows.insert(window, object);
    part.pending.insert(window, PendingHints::none());
    Vec::new()
}

/// `set_first_mouse` (double-buffered: the model hears it at the toplevel's next commit).
pub fn on_set_first_mouse(
    part: &mut QuireWindowPart,
    window: WindowId,
    first_mouse: Vec<LogicalRect>,
) -> Vec<Event> {
    stage(part, window, |pending| {
        pending.with_first_mouse(first_mouse)
    });
    Vec::new()
}

/// `set_frame` (double-buffered, like `set_first_mouse`).
pub fn on_set_frame(part: &mut QuireWindowPart, window: WindowId, frame: FrameHint) -> Vec<Event> {
    stage(part, window, |pending| pending.with_frame(frame));
    Vec::new()
}

/// Runs `f` on `window`'s staged values.
fn stage(
    part: &mut QuireWindowPart,
    window: WindowId,
    f: impl FnOnce(PendingHints) -> PendingHints,
) {
    let staged = part.pending.remove(&window).unwrap_or_default();
    part.pending.insert(window, f(staged));
}

/// The toplevel committed: what it staged takes effect.
pub fn on_commit(part: &mut QuireWindowPart, window: WindowId) -> Vec<Event> {
    match part.pending.get_mut(&window) {
        Some(staged) => std::mem::take(staged)
            .commit()
            .into_iter()
            .map(|hint| Event::WindowHint { window, hint })
            .collect(),
        None => Vec::new(),
    }
}

fn hint(window: WindowId, hint: WindowHintChange) -> Vec<Event> {
    vec![Event::WindowHint { window, hint }]
}

/// `set_role` (`dialog` and `shell` are refused with `bad_role` by the dispatch).
pub fn on_set_role(_part: &mut QuireWindowPart, window: WindowId, role: WindowRole) -> Vec<Event> {
    hint(window, WindowHintChange::Role(role))
}

/// `set_a11y_root`.
pub fn on_set_a11y_root(
    _part: &mut QuireWindowPart,
    window: WindowId,
    root: Option<A11yRoot>,
) -> Vec<Event> {
    hint(window, WindowHintChange::A11y(root))
}

/// `set_edited`.
pub fn on_set_edited(_part: &mut QuireWindowPart, window: WindowId, edited: Edited) -> Vec<Event> {
    hint(window, WindowHintChange::Edited(edited))
}

/// `request_placement`, answering the press `serial` (the dispatch has already checked the
/// press): stored, and from P2 acted on ([`PLACEMENT_POLICY`]).
pub fn on_request_placement(
    part: &mut QuireWindowPart,
    window: WindowId,
    target: PlaceTarget,
    serial: InputSerial,
) -> Vec<Event> {
    part.placements.insert(window, (target, serial));
    placement_events(PLACEMENT_POLICY, window, target, serial)
}

/// Sends `window` its emphasis (from focus).
pub fn apply_emphasis(
    part: &mut QuireWindowPart,
    window: WindowId,
    emphasis: Emphasis,
) -> Result<(), WaylandFault> {
    match part.windows.get(&window) {
        Some(object) if object.is_alive() => {
            send_emphasis(object, emphasis);
            Ok(())
        }
        Some(_) => Err(WaylandFault::SurfaceGone),
        // A window that never extended itself has nobody to tell.
        None => Ok(()),
    }
}

/// Sends `window` its placement mark when it differs from the one it was last told (the model
/// publishes a window whenever its state changes, and most changes leave the mark alone).
///
/// ```no_run
/// use casement_core::WindowId;
/// use casement_protocols::vocab::PlacementMark;
/// use casement_wayland::quire_window::{QuireWindowPart, apply_placement};
///
/// # fn demo(part: &mut QuireWindowPart) {
/// // Nobody extended window 1: there is nobody to tell.
/// assert!(apply_placement(part, WindowId(1), PlacementMark::Left).is_ok());
/// # }
/// ```
pub fn apply_placement(
    part: &mut QuireWindowPart,
    window: WindowId,
    mark: PlacementMark,
) -> Result<(), WaylandFault> {
    match part.windows.get(&window) {
        Some(object) if object.is_alive() => {
            if part.marks.insert(window, mark) != Some(mark) {
                send_placement(object, mark);
            }
            Ok(())
        }
        Some(_) => Err(WaylandFault::SurfaceGone),
        None => Ok(()),
    }
}

/// Tells `client` the person asked its app to quit: `quit` on each manager it holds at
/// version 2 or later. A client with none (or none alive) has nobody to tell.
pub fn apply_quit(part: &mut QuireWindowPart, client: ClientKey) -> Result<(), WaylandFault> {
    let live: Vec<&QuireWindowManagerV1> = part
        .quit_managers
        .iter()
        .filter(|(held, manager)| *held == client && manager.is_alive())
        .map(|(_, manager)| manager)
        .collect();
    if live.is_empty() {
        return Err(WaylandFault::SurfaceGone);
    }
    for manager in live {
        manager.quit();
    }
    Ok(())
}
