//! `zwlr_foreign_toplevel_manager_v1`: the window list for docks and switchers that speak the
//! wlr protocol (sill's `desktop_wl` Wlr arm). Every window is listed with its title, app id,
//! state and the outputs it is on; the handle's requests become `Event::WlrToplevel`.
//!
//! smithay has no module for it, so this one carries its own `GlobalDispatch2` and `Dispatch2`
//! impls (as `quire_window` does); `delegate_dispatch2!` in the composition root routes to
//! them and they reach the root through [`WlrToplevelHandler`].
//!
//! The part keeps the facts last published for each window, and for each handle what that
//! handle was last told, so a handle is brought up to date with exactly the events it lacks and
//! a manager that binds late is sent every window as it now is. What each state means is in
//! [`states_of`] and [`Diff`], pure; this file is the wire.

use crate::foreign_toplevel_list::Disclosure;
use crate::limits::{self, Limit, Standing};
use crate::states::{GlobalFilter, WaylandFault};
use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize};
use casement_core::policy::GlobalKind;
use casement_core::window::{Window, WindowState, WlrToplevelRequest};
use casement_core::{Event, OutputKey, WindowId};
use casement_protocols::vocab::{Emphasis, Toggle};
use smithay::output::Output;
use smithay::reexports::wayland_protocols_wlr::foreign_toplevel::v1::server::{
    zwlr_foreign_toplevel_handle_v1::{self, ZwlrForeignToplevelHandleV1},
    zwlr_foreign_toplevel_manager_v1::{self, ZwlrForeignToplevelManagerV1},
};
use smithay::reexports::wayland_server::backend::{ClientId, GlobalId};
use smithay::reexports::wayland_server::protocol::wl_output::WlOutput;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::{
    Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch, New, Resource,
};
use smithay::wayland::{Dispatch2, GlobalDispatch2};
use std::collections::BTreeMap;

/// The version casement serves.
pub const VERSION: u32 = 3;

/// `zwlr_foreign_toplevel_manager_v1`, the windows listed on it and the handle each client holds.
#[derive(Debug, Default)]
pub struct WlrToplevelPart {
    global: Option<GlobalId>,
    managers: Vec<ZwlrForeignToplevelManagerV1>,
    /// What was last published for each listed window.
    facts: BTreeMap<WindowId, Facts>,
    /// Every handle of each window, and what it was told.
    handles: BTreeMap<WindowId, Vec<Held>>,
    spawn: Option<Spawn>,
}

/// How to make a handle where the state type is known ([`WlrToplevelPart::new`]).
#[derive(Debug, Clone)]
struct Spawn {
    display: DisplayHandle,
    make: fn(
        &DisplayHandle,
        &ZwlrForeignToplevelManagerV1,
        WindowId,
    ) -> Option<ZwlrForeignToplevelHandleV1>,
}

/// One window as a list client sees it.
#[derive(Debug, Clone, PartialEq)]
struct Facts {
    title: String,
    app_id: String,
    states: Vec<u32>,
    outputs: Vec<Output>,
}

/// A handle and what it was last told (nothing yet for a handle just made).
#[derive(Debug)]
struct Held {
    handle: ZwlrForeignToplevelHandleV1,
    told: Option<Facts>,
}

impl WlrToplevelPart {
    /// Creates the global (version 3), filtered by the I7 table.
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> WlrToplevelPart
    where
        D: GlobalDispatch<ZwlrForeignToplevelManagerV1, WlrToplevelGlobal>
            + Dispatch<ZwlrForeignToplevelHandleV1, WlrHandleData>
            + 'static,
    {
        WlrToplevelPart {
            global: Some(display.create_global::<D, ZwlrForeignToplevelManagerV1, _>(
                VERSION,
                WlrToplevelGlobal {
                    filter: role_filter,
                },
            )),
            managers: Vec::new(),
            facts: BTreeMap::new(),
            handles: BTreeMap::new(),
            spawn: Some(Spawn {
                display: display.clone(),
                make: |display, manager, window| {
                    let client = manager.client()?;
                    let handle = client
                        .create_resource::<ZwlrForeignToplevelHandleV1, WlrHandleData, D>(
                            display,
                            manager.version(),
                            WlrHandleData { window },
                        )
                        .ok()?;
                    manager.toplevel(&handle);
                    Some(handle)
                },
            }),
        }
    }

    /// How many managers `client` has bound.
    pub fn managers_of(&self, client: &ClientId) -> usize {
        self.managers
            .iter()
            .filter(|manager| manager.client().is_some_and(|owner| owner.id() == *client))
            .count()
    }

    /// The global, when it was created.
    pub fn global(&self) -> Option<&GlobalId> {
        self.global.as_ref()
    }

    /// Whether `window` is listed.
    pub fn is_listed(&self, window: WindowId) -> bool {
        self.facts.contains_key(&window)
    }
}

/// What the composition root provides to this protocol's dispatch.
pub trait WlrToplevelHandler: 'static {
    /// The part.
    fn wlr_toplevel_part(&mut self) -> &mut WlrToplevelPart;
    /// Feeds the events a request produced into the model (`core_loop::feed`).
    fn wlr_toplevel_events(&mut self, events: Vec<Event>);
    /// Where `surface` sits in the global logical space, when casement placed it itself (a
    /// layer surface); `None` for a surface with no known place.
    fn surface_origin(&self, surface: &WlSurface) -> Option<LogicalPoint>;
    /// The output a client's `wl_output` is.
    fn output_key_of(&self, output: &WlOutput) -> Option<OutputKey>;
}

/// The global's data: the filter its `can_view` asks.
#[derive(Debug, Clone, Copy)]
pub struct WlrToplevelGlobal {
    filter: GlobalFilter,
}

/// A bound manager's data.
#[derive(Debug, Clone, Copy, Default)]
pub struct WlrManagerData;

/// One handle's data: the window it stands for.
#[derive(Debug, Clone, Copy)]
pub struct WlrHandleData {
    /// The window.
    pub window: WindowId,
}

impl<D> GlobalDispatch2<ZwlrForeignToplevelManagerV1, D> for WlrToplevelGlobal
where
    D: WlrToplevelHandler + Dispatch<ZwlrForeignToplevelManagerV1, WlrManagerData> + 'static,
{
    fn bind(
        &self,
        state: &mut D,
        handle: &DisplayHandle,
        client: &Client,
        resource: New<ZwlrForeignToplevelManagerV1>,
        data_init: &mut DataInit<'_, D>,
    ) {
        let manager = data_init.init(resource, WlrManagerData);
        let part = state.wlr_toplevel_part();
        // Each bind lists every window again and hears every title change: a client that binds
        // past its cap is cut (CASW-06) and gets no listing.
        let held = part.managers_of(&client.id());
        if limits::admit(handle, client, Limit::WindowListManagers, held) == Standing::Over {
            return;
        }
        part.managers.push(manager.clone());
        list_all_on(part, &manager);
    }

    fn can_view(&self, client: &Client) -> bool {
        self.filter.sees(GlobalKind::WlrToplevel, client)
    }
}

impl<D> Dispatch2<ZwlrForeignToplevelManagerV1, D> for WlrManagerData
where
    D: WlrToplevelHandler,
{
    fn request(
        &self,
        state: &mut D,
        _client: &Client,
        resource: &ZwlrForeignToplevelManagerV1,
        request: zwlr_foreign_toplevel_manager_v1::Request,
        _dhandle: &DisplayHandle,
        _data_init: &mut DataInit<'_, D>,
    ) {
        if let zwlr_foreign_toplevel_manager_v1::Request::Stop = request {
            state
                .wlr_toplevel_part()
                .managers
                .retain(|manager| manager != resource);
            resource.finished();
        }
    }

    fn destroyed(&self, state: &mut D, _client: ClientId, resource: &ZwlrForeignToplevelManagerV1) {
        state
            .wlr_toplevel_part()
            .managers
            .retain(|manager| manager != resource);
    }
}

impl<D> Dispatch2<ZwlrForeignToplevelHandleV1, D> for WlrHandleData
where
    D: WlrToplevelHandler,
{
    fn request(
        &self,
        state: &mut D,
        _client: &Client,
        resource: &ZwlrForeignToplevelHandleV1,
        request: zwlr_foreign_toplevel_handle_v1::Request,
        _dhandle: &DisplayHandle,
        _data_init: &mut DataInit<'_, D>,
    ) {
        use zwlr_foreign_toplevel_handle_v1::Request as Asked;
        let window = self.window;
        if !state.wlr_toplevel_part().is_listed(window) {
            return;
        }
        let asked = match request {
            Asked::SetMaximized => Some(WlrToplevelRequest::Maximise(Toggle::On)),
            Asked::UnsetMaximized => Some(WlrToplevelRequest::Maximise(Toggle::Off)),
            Asked::SetMinimized => Some(WlrToplevelRequest::Minimise(Toggle::On)),
            Asked::UnsetMinimized => Some(WlrToplevelRequest::Minimise(Toggle::Off)),
            Asked::Activate { .. } => Some(WlrToplevelRequest::Activate),
            Asked::Close => Some(WlrToplevelRequest::Close),
            Asked::SetRectangle {
                surface,
                x,
                y,
                width,
                height,
            } => match rectangle_of(state.surface_origin(&surface), (x, y, width, height)) {
                Some(rect) => Some(WlrToplevelRequest::MinimiseRect(rect)),
                None => {
                    resource.post_error(
                        zwlr_foreign_toplevel_handle_v1::Error::InvalidRectangle,
                        "a rectangle has no negative size",
                    );
                    None
                }
            },
            Asked::SetFullscreen { output } => Some(WlrToplevelRequest::FullScreen {
                toggle: Toggle::On,
                output: output.and_then(|output| state.output_key_of(&output)),
            }),
            Asked::UnsetFullscreen => Some(WlrToplevelRequest::FullScreen {
                toggle: Toggle::Off,
                output: None,
            }),
            _ => None,
        };
        if let Some(asked) = asked {
            let events = on_request(state.wlr_toplevel_part(), window, asked);
            state.wlr_toplevel_events(events);
        }
    }

    fn destroyed(&self, state: &mut D, _client: ClientId, resource: &ZwlrForeignToplevelHandleV1) {
        if let Some(held) = state.wlr_toplevel_part().handles.get_mut(&self.window) {
            held.retain(|held| &held.handle != resource);
        }
    }
}

/// The rectangle a `set_rectangle` names, in global logical coordinates: `rect` is in the
/// surface's own, `origin` is where the surface sits (the origin when casement does not know).
/// `None` for a negative size, which the protocol makes an error.
pub fn rectangle_of(
    origin: Option<LogicalPoint>,
    rect: (i32, i32, i32, i32),
) -> Option<LogicalRect> {
    let (x, y, width, height) = rect;
    let (w, h) = (u32::try_from(width).ok()?, u32::try_from(height).ok()?);
    let origin = origin.unwrap_or(LogicalPoint { x: 0, y: 0 });
    Some(LogicalRect::new(
        LogicalPoint {
            x: origin.x.saturating_add(x),
            y: origin.y.saturating_add(y),
        },
        LogicalSize { w, h },
    ))
}

/// A request on `window`'s handle.
///
/// ```no_run
/// use casement_core::WindowId;
/// use casement_core::window::WlrToplevelRequest;
/// use casement_wayland::wlr_toplevel::{WlrToplevelPart, on_request};
///
/// let mut part = WlrToplevelPart::default();
/// let events = on_request(&mut part, WindowId(1), WlrToplevelRequest::Close);
/// assert_eq!(events.len(), 1);
/// ```
pub fn on_request(
    part: &mut WlrToplevelPart,
    window: WindowId,
    request: WlrToplevelRequest,
) -> Vec<Event> {
    let _ = part;
    vec![Event::WlrToplevel { window, request }]
}

/// The states `window` is in, as the protocol numbers them (`maximized` 0, `minimized` 1,
/// `activated` 2, `fullscreen` 3), in ascending order.
pub fn states_of(window: &Window) -> Vec<u32> {
    let maximised = matches!(window.state, WindowState::Filled { .. });
    let minimised = matches!(window.state, WindowState::Minimised { .. });
    let full_screen = matches!(window.state, WindowState::FullScreen { .. });
    [
        (0, maximised),
        (1, minimised),
        (2, window.emphasis == Emphasis::Key),
        (3, full_screen),
    ]
    .into_iter()
    .filter(|(_, held)| *held)
    .map(|(code, _)| code)
    .collect()
}

/// What a handle lacks of the facts: the pure part of bringing it up to date.
#[derive(Debug, Default, PartialEq)]
struct Diff {
    title: Option<String>,
    app_id: Option<String>,
    states: Option<Vec<u32>>,
    entered: Vec<Output>,
    left: Vec<Output>,
}

impl Diff {
    /// What `told` lacks of `now`; everything when it was told nothing.
    fn between(told: Option<&Facts>, now: &Facts) -> Diff {
        let changed = |was: Option<&String>, is: &String| (was != Some(is)).then(|| is.clone());
        Diff {
            title: changed(told.map(|t| &t.title), &now.title),
            app_id: changed(told.map(|t| &t.app_id), &now.app_id),
            states: (told.map(|t| &t.states) != Some(&now.states)).then(|| now.states.clone()),
            entered: now
                .outputs
                .iter()
                .filter(|output| told.is_none_or(|t| !t.outputs.contains(output)))
                .cloned()
                .collect(),
            left: told
                .into_iter()
                .flat_map(|t| t.outputs.iter())
                .filter(|output| !now.outputs.contains(output))
                .cloned()
                .collect(),
        }
    }

    /// Whether there is nothing to send.
    fn is_quiet(&self) -> bool {
        *self == Diff::default()
    }
}

/// Sends `held` what it lacks of `facts`, then `done`.
fn bring_up_to_date(held: &mut Held, facts: &Facts) {
    let diff = Diff::between(held.told.as_ref(), facts);
    if diff.is_quiet() {
        return;
    }
    let handle = &held.handle;
    if let Some(title) = diff.title {
        handle.title(title);
    }
    if let Some(app_id) = diff.app_id {
        handle.app_id(app_id);
    }
    let wire_outputs = |output: &Output| -> Vec<WlOutput> {
        handle
            .client()
            .map(|client| output.client_outputs(&client).collect())
            .unwrap_or_default()
    };
    for output in &diff.left {
        for wl in wire_outputs(output) {
            handle.output_leave(&wl);
        }
    }
    for output in &diff.entered {
        for wl in wire_outputs(output) {
            handle.output_enter(&wl);
        }
    }
    if let Some(states) = diff.states {
        handle.state(states.iter().flat_map(|s| s.to_ne_bytes()).collect());
    }
    handle.done();
    held.told = Some(facts.clone());
}

/// Lists every window already published to a manager that just bound.
fn list_all_on(part: &mut WlrToplevelPart, manager: &ZwlrForeignToplevelManagerV1) {
    let WlrToplevelPart {
        facts,
        handles,
        spawn,
        ..
    } = part;
    let Some(spawn) = spawn else {
        return;
    };
    for (window, facts) in facts.iter() {
        if let Some(handle) = (spawn.make)(&spawn.display, manager, *window) {
            let mut held = Held { handle, told: None };
            bring_up_to_date(&mut held, facts);
            handles.entry(*window).or_default().push(held);
        }
    }
}

/// Lists `window` (or sends what changed: title, app id, state, the outputs it is on), then
/// `done`. `outputs` are the ones it is on.
pub fn apply_toplevel(
    part: &mut WlrToplevelPart,
    window: &Window,
    outputs: &[Output],
    disclosure: Disclosure,
) -> Result<(), WaylandFault> {
    let facts = Facts {
        title: disclosure.text(&window.title.0).to_owned(),
        app_id: disclosure.text(window.claimed_id.as_str()).to_owned(),
        states: states_of(window),
        outputs: outputs.to_vec(),
    };
    let known = part.facts.insert(window.id, facts.clone()).is_some();
    if !known {
        let made: Vec<ZwlrForeignToplevelHandleV1> = match &part.spawn {
            Some(spawn) => part
                .managers
                .iter()
                .filter_map(|manager| (spawn.make)(&spawn.display, manager, window.id))
                .collect(),
            None => Vec::new(),
        };
        part.handles
            .entry(window.id)
            .or_default()
            .extend(made.into_iter().map(|handle| Held { handle, told: None }));
    }
    for held in part.handles.entry(window.id).or_default().iter_mut() {
        bring_up_to_date(held, &facts);
    }
    Ok(())
}

/// Withdraws `window`: its handles get `closed`.
pub fn apply_closed(part: &mut WlrToplevelPart, window: WindowId) -> Result<(), WaylandFault> {
    part.facts
        .remove(&window)
        .ok_or(WaylandFault::UnknownWindow(window))?;
    for held in part.handles.remove(&window).unwrap_or_default() {
        held.handle.closed();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(title: &str, states: Vec<u32>) -> Facts {
        Facts {
            title: title.to_owned(),
            app_id: "org.example.Mail".to_owned(),
            states,
            outputs: Vec::new(),
        }
    }

    #[test]
    fn a_new_handle_lacks_everything_and_an_unchanged_one_nothing() {
        let now = facts("Inbox", vec![2]);
        let first = Diff::between(None, &now);
        assert_eq!(first.title.as_deref(), Some("Inbox"));
        assert_eq!(first.app_id.as_deref(), Some("org.example.Mail"));
        assert_eq!(first.states, Some(vec![2]));
        assert!(Diff::between(Some(&now), &now).is_quiet());
    }

    #[test]
    fn only_what_changed_is_sent() {
        let was = facts("Inbox", vec![2]);
        let diff = Diff::between(Some(&was), &facts("Drafts", vec![2]));
        assert_eq!(diff.title.as_deref(), Some("Drafts"));
        assert_eq!(diff.app_id, None);
        assert_eq!(diff.states, None);
        let diff = Diff::between(Some(&was), &facts("Inbox", vec![1]));
        assert_eq!(diff.states, Some(vec![1]));
        assert_eq!(diff.title, None);
    }

    #[test]
    fn a_rectangle_is_moved_by_its_surface_and_a_negative_size_is_refused() {
        let at = Some(LogicalPoint { x: 100, y: 1000 });
        assert_eq!(
            rectangle_of(at, (10, 20, 64, 48)),
            Some(LogicalRect::new(
                LogicalPoint { x: 110, y: 1020 },
                LogicalSize { w: 64, h: 48 }
            ))
        );
        assert_eq!(
            rectangle_of(None, (5, 6, 0, 0)).map(|r| r.origin),
            Some(LogicalPoint { x: 5, y: 6 })
        );
        assert_eq!(rectangle_of(at, (0, 0, -1, 4)), None);
        assert_eq!(rectangle_of(at, (0, 0, 4, -1)), None);
    }
}
