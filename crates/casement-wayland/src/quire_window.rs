//! `quire_window_manager_v1` (C50): what an app says about its toplevel (first-mouse regions,
//! frame, role, accessibility root, edited) and what it learns back (emphasis, placement).
//! Advertised from P0 so shell-host can tell casement apart by this global; requests are
//! stored in the model, `request_placement` is acted on from P2.
//!
//! smithay has no module for our protocol, so this one carries its own `GlobalDispatch2` and
//! `Dispatch2` impls; `delegate_dispatch2!` in the composition root routes to them, and they
//! reach the root through [`QuireWindowHandler`].

use crate::states::{GlobalFilter, WaylandFault};
use casement_core::policy::GlobalKind;
use casement_core::window::{A11yRoot, FirstMouse, FrameHint};
use casement_core::{Event, InputSerial, WindowId};
use casement_protocols::server::quire_window_v1::quire_window_manager_v1::{
    self, QuireWindowManagerV1,
};
use casement_protocols::server::quire_window_v1::quire_window_v1::{self, QuireWindowV1};
use casement_protocols::vocab::{Edited, Emphasis, PlaceTarget, PlacementMark, WindowRole};
use smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel::XdgToplevel;
use smithay::reexports::wayland_server::backend::{ClientId, GlobalId};
use smithay::reexports::wayland_server::{
    Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch, New,
};
use smithay::wayland::{Dispatch2, GlobalDispatch2};
use std::collections::BTreeMap;

/// The version casement serves.
pub const VERSION: u32 = 1;

/// `quire_window_manager_v1` and its per-toplevel objects.
#[derive(Debug)]
pub struct QuireWindowPart {
    /// The global.
    pub global: GlobalId,
    /// Each extended toplevel's object (at most one per window).
    pub windows: BTreeMap<WindowId, QuireWindowV1>,
}

impl QuireWindowPart {
    /// Creates the global (version 1), filtered by the I7 table.
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
        }
    }
}

/// What the composition root provides to this protocol's dispatch.
pub trait QuireWindowHandler: 'static {
    /// The part.
    fn quire_window_part(&mut self) -> &mut QuireWindowPart;
    /// The window an `xdg_toplevel` is, if it is live.
    fn window_of_toplevel(&self, toplevel: &XdgToplevel) -> Option<WindowId>;
    /// Feeds the events a request produced into the model (`core_loop::feed`).
    fn quire_window_events(&mut self, events: Vec<Event>);
}

/// The global's data: the filter its `can_view` asks.
#[derive(Debug, Clone, Copy)]
pub struct QuireWindowGlobal {
    filter: GlobalFilter,
}

/// A bound manager's data.
#[derive(Debug, Clone, Copy, Default)]
pub struct QuireWindowManagerData;

/// One extension object's data: the window it extends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuireWindowData {
    /// The extended window (`None` once its toplevel is gone: the object is defunct).
    pub window: Option<WindowId>,
}

impl<D> GlobalDispatch2<QuireWindowManagerV1, D> for QuireWindowGlobal
where
    D: Dispatch<QuireWindowManagerV1, QuireWindowManagerData> + 'static,
{
    fn bind(
        &self,
        _state: &mut D,
        _handle: &DisplayHandle,
        _client: &Client,
        resource: New<QuireWindowManagerV1>,
        data_init: &mut DataInit<'_, D>,
    ) {
        data_init.init(resource, QuireWindowManagerData);
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
        client: &Client,
        resource: &QuireWindowManagerV1,
        request: quire_window_manager_v1::Request,
        dhandle: &DisplayHandle,
        data_init: &mut DataInit<'_, D>,
    ) {
        let _ = (state, client, resource, request, dhandle, data_init);
        todo!("P0-H2: quire_window_manager_v1 requests (get_window, destroy)")
    }
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
        dhandle: &DisplayHandle,
        data_init: &mut DataInit<'_, D>,
    ) {
        let _ = (state, client, resource, request, dhandle, data_init);
        todo!("P0-H2: quire_window_v1 requests")
    }

    fn destroyed(&self, state: &mut D, client: ClientId, resource: &QuireWindowV1) {
        let _ = (state, client, resource);
        todo!("P0-H2: quire_window_v1 destroyed")
    }
}

/// `get_window`: `window`'s toplevel was extended with `object`.
pub fn on_get_window(
    part: &mut QuireWindowPart,
    window: WindowId,
    object: QuireWindowV1,
) -> Vec<Event> {
    let _ = (part, window, object);
    todo!("P0-H2: quire_window::on_get_window")
}

/// `set_first_mouse`.
pub fn on_set_first_mouse(
    part: &mut QuireWindowPart,
    window: WindowId,
    first_mouse: FirstMouse,
) -> Vec<Event> {
    let _ = (part, window, first_mouse);
    todo!("P0-H2: quire_window::on_set_first_mouse")
}

/// `set_frame`.
pub fn on_set_frame(part: &mut QuireWindowPart, window: WindowId, frame: FrameHint) -> Vec<Event> {
    let _ = (part, window, frame);
    todo!("P0-H2: quire_window::on_set_frame")
}

/// `set_role` (`dialog` and `shell` are refused with `bad_role` by the dispatch).
pub fn on_set_role(part: &mut QuireWindowPart, window: WindowId, role: WindowRole) -> Vec<Event> {
    let _ = (part, window, role);
    todo!("P0-H2: quire_window::on_set_role")
}

/// `set_a11y_root`.
pub fn on_set_a11y_root(
    part: &mut QuireWindowPart,
    window: WindowId,
    root: A11yRoot,
) -> Vec<Event> {
    let _ = (part, window, root);
    todo!("P0-H2: quire_window::on_set_a11y_root")
}

/// `set_edited`.
pub fn on_set_edited(part: &mut QuireWindowPart, window: WindowId, edited: Edited) -> Vec<Event> {
    let _ = (part, window, edited);
    todo!("P0-H2: quire_window::on_set_edited")
}

/// `request_placement`, answering the press `serial` (stored at P0; acted on from P2).
pub fn on_request_placement(
    part: &mut QuireWindowPart,
    window: WindowId,
    target: PlaceTarget,
    serial: InputSerial,
) -> Vec<Event> {
    let _ = (part, window, target, serial);
    todo!("P0-H2: quire_window::on_request_placement")
}

/// Sends `window` its emphasis (from focus).
pub fn apply_emphasis(
    part: &mut QuireWindowPart,
    window: WindowId,
    emphasis: Emphasis,
) -> Result<(), WaylandFault> {
    let _ = (part, window, emphasis);
    todo!("P0-H2: quire_window::apply_emphasis")
}

/// Sends `window` its placement mark.
pub fn apply_placement(
    part: &mut QuireWindowPart,
    window: WindowId,
    mark: PlacementMark,
) -> Result<(), WaylandFault> {
    let _ = (part, window, mark);
    todo!("P0-H2: quire_window::apply_placement")
}
