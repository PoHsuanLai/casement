//! `quire_shell_v1`: the shell's surface roles (which layer surface is the bar, the dock, the
//! switcher), the minimise targets the dock gives its tiles, and the events only the
//! compositor sees (the real modifier keys, hot corners, edge reveals). Shell connections
//! only. Frozen at P2-Z; filled by P2-Q2.
//!
//! The wire glue is written: the part, the handler trait, the global's and the object's
//! dispatch, and the errors a refusal becomes. What each request decides (`on_*`) and what a
//! note sends (`apply_note`) are the lane's. The switcher, overview and show-desktop requests
//! are ignored until P3-Z gives them their machines.

use super::{Advertised, code_of};
use crate::states::{GlobalFilter, WaylandFault};
use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize};
use casement_core::policy::GlobalKind;
use casement_core::shell::ShellNote;
use casement_core::{Event, LayerKey};
use casement_protocols::server::quire_shell_v1::quire_shell_v1::{self, QuireShellV1};
use casement_protocols::vocab::{ShellRole, WireCode};
use smithay::reexports::wayland_server::backend::{ClientId, GlobalId};
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::{
    Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch, New, Resource,
};
use smithay::wayland::{Dispatch2, GlobalDispatch2};
use std::collections::BTreeMap;

/// The version casement serves.
pub const VERSION: u32 = 1;

/// Whether the global is created yet (the freeze: no; P2-Q2 flips it).
pub const GLOBAL: Advertised = Advertised::Not;

/// The surface that holds a role.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleHolder {
    /// The layer surface's key in the model.
    pub layer: LayerKey,
    /// The surface (the target of the events addressed to the role).
    pub surface: WlSurface,
}

/// `quire_shell_v1` and the objects bound on it.
#[derive(Debug)]
pub struct QuireShellPart {
    /// The global, once it is created ([`GLOBAL`]).
    pub global: Option<GlobalId>,
    /// Every bound `quire_shell_v1`.
    pub bound: Vec<QuireShellV1>,
    /// The surface holding each role.
    pub roles: BTreeMap<ShellRole, RoleHolder>,
}

impl QuireShellPart {
    /// The part; creates the global (version 1) behind the I7 filter when [`GLOBAL`] says so.
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> QuireShellPart
    where
        D: GlobalDispatch<QuireShellV1, ShellGlobal> + 'static,
    {
        let global = match GLOBAL {
            Advertised::Yes => Some(display.create_global::<D, QuireShellV1, _>(
                VERSION,
                ShellGlobal {
                    filter: role_filter,
                },
            )),
            Advertised::Not => None,
        };
        QuireShellPart {
            global,
            bound: Vec::new(),
            roles: BTreeMap::new(),
        }
    }
}

/// What the composition root provides to this protocol's dispatch.
pub trait QuireShellHandler: 'static {
    /// The part.
    fn quire_shell_part(&mut self) -> &mut QuireShellPart;
    /// Feeds the events a request produced into the model (`core_loop::feed`).
    fn quire_shell_events(&mut self, events: Vec<Event>);
    /// The layer surface `surface` is, when it is a live one of `client`.
    fn layer_of_surface(&self, client: &Client, surface: &WlSurface) -> Option<LayerKey>;
}

/// The global's data: the filter its `can_view` asks.
#[derive(Debug, Clone, Copy)]
pub struct ShellGlobal {
    filter: GlobalFilter,
}

/// A bound object's data.
#[derive(Debug, Clone, Copy, Default)]
pub struct ShellData;

/// Why `set_role` was refused (a protocol error to the client).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RoleRefusal {
    /// The surface is not a layer surface of this client (`bad_surface`).
    NotALayer,
    /// Another live surface holds the role (`role_taken`).
    Taken,
}

/// Why a minimise target was refused (a protocol error to the client).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TargetRefusal {
    /// The dock is not a layer surface of this client (`bad_surface`).
    NotALayer,
    /// No toplevel has the identifier (`unknown_toplevel`).
    UnknownToplevel,
}

impl<D> GlobalDispatch2<QuireShellV1, D> for ShellGlobal
where
    D: QuireShellHandler + Dispatch<QuireShellV1, ShellData> + 'static,
{
    fn bind(
        &self,
        state: &mut D,
        _handle: &DisplayHandle,
        _client: &Client,
        resource: New<QuireShellV1>,
        data_init: &mut DataInit<'_, D>,
    ) {
        let shell = data_init.init(resource, ShellData);
        state.quire_shell_part().bound.push(shell);
    }

    fn can_view(&self, client: &Client) -> bool {
        self.filter.sees(GlobalKind::QuireShell, client)
    }
}

impl<D> Dispatch2<QuireShellV1, D> for ShellData
where
    D: QuireShellHandler,
{
    fn request(
        &self,
        state: &mut D,
        client: &Client,
        resource: &QuireShellV1,
        request: quire_shell_v1::Request,
        _dhandle: &DisplayHandle,
        _data_init: &mut DataInit<'_, D>,
    ) {
        match request {
            quire_shell_v1::Request::SetRole { surface, role } => {
                let Some(role) = code_of(role).and_then(|code| ShellRole::from_code(code).ok())
                else {
                    return;
                };
                let layer = state.layer_of_surface(client, &surface);
                match on_set_role(state.quire_shell_part(), layer, &surface, role) {
                    Ok(events) => state.quire_shell_events(events),
                    Err(RoleRefusal::NotALayer) => resource.post_error(
                        quire_shell_v1::Error::BadSurface,
                        "the surface is not a layer surface of this client",
                    ),
                    Err(RoleRefusal::Taken) => resource.post_error(
                        quire_shell_v1::Error::RoleTaken,
                        "another surface already has that role",
                    ),
                }
            }
            quire_shell_v1::Request::SetMinimiseTarget {
                dock,
                toplevel,
                x,
                y,
                width,
                height,
            } => {
                let dock = state.layer_of_surface(client, &dock);
                let local = minimise_rect(x, y, width, height);
                match on_set_minimise_target(state.quire_shell_part(), dock, &toplevel, local) {
                    Ok(events) => state.quire_shell_events(events),
                    Err(refusal) => refuse_target(resource, refusal),
                }
            }
            quire_shell_v1::Request::ClearMinimiseTarget { toplevel } => {
                match on_clear_minimise_target(state.quire_shell_part(), &toplevel) {
                    Ok(events) => state.quire_shell_events(events),
                    Err(refusal) => refuse_target(resource, refusal),
                }
            }
            // The switcher, overview and show-desktop requests arrive with P3-Z.
            _ => {}
        }
    }

    fn destroyed(&self, state: &mut D, _client: ClientId, resource: &QuireShellV1) {
        on_shell_gone(state.quire_shell_part(), resource);
    }
}

/// The protocol error a refused minimise target is.
fn refuse_target(resource: &QuireShellV1, refusal: TargetRefusal) {
    match refusal {
        TargetRefusal::NotALayer => resource.post_error(
            quire_shell_v1::Error::BadSurface,
            "the dock is not a layer surface of this client",
        ),
        TargetRefusal::UnknownToplevel => resource.post_error(
            quire_shell_v1::Error::UnknownToplevel,
            "no toplevel has that identifier",
        ),
    }
}

/// `set_role`: `surface` (the layer `layer`, when it is one of the client's) takes `role`.
/// Refuses a surface that is no layer surface of the client and a role another live surface
/// holds; otherwise the request the model takes (`Event::Shell(ShellRequest::Role)`).
pub fn on_set_role(
    part: &mut QuireShellPart,
    layer: Option<LayerKey>,
    surface: &WlSurface,
    role: ShellRole,
) -> Result<Vec<Event>, RoleRefusal> {
    let _ = (part, layer, surface, role);
    todo!("P2-Q2: on_set_role")
}

/// `set_minimise_target`: where the window `toplevel` (a `ext_foreign_toplevel_list`
/// identifier) shrinks into, `local` to the `dock` layer surface.
pub fn on_set_minimise_target(
    part: &mut QuireShellPart,
    dock: Option<LayerKey>,
    toplevel: &str,
    local: LogicalRect,
) -> Result<Vec<Event>, TargetRefusal> {
    let _ = (part, dock, toplevel, local);
    todo!("P2-Q2: on_set_minimise_target")
}

/// `clear_minimise_target`: forget the target of the window `toplevel`.
pub fn on_clear_minimise_target(
    part: &mut QuireShellPart,
    toplevel: &str,
) -> Result<Vec<Event>, TargetRefusal> {
    let _ = (part, toplevel);
    todo!("P2-Q2: on_clear_minimise_target")
}

/// A bound object went away: the roles it left are not forgotten (the surfaces stay until
/// they unmap), only the object is.
pub fn on_shell_gone(part: &mut QuireShellPart, shell: &QuireShellV1) {
    part.bound.retain(|bound| bound != shell);
}

/// A layer surface unmapped: whatever role it held is free again.
pub fn on_layer_gone(part: &mut QuireShellPart, layer: LayerKey) {
    part.roles.retain(|_, holder| holder.layer != layer);
}

/// The model said `note`: record it (a role taken, a minimise target) or send it to the
/// shell (`modifiers` to the role surfaces, `hot_corner`, `reveal`).
pub fn apply_note(part: &mut QuireShellPart, note: &ShellNote) -> Result<(), WaylandFault> {
    let _ = (part, note);
    todo!("P2-Q2: apply_note")
}

/// A `LogicalRect` from the wire's four integers, as the dispatch builds it.
///
/// ```
/// use casement_wayland::quire::shell::minimise_rect;
/// let rect = minimise_rect(10, 5, 64, 48);
/// assert_eq!((rect.origin.x, rect.size.w), (10, 64));
/// ```
pub fn minimise_rect(x: i32, y: i32, width: u32, height: u32) -> LogicalRect {
    LogicalRect::new(
        LogicalPoint { x, y },
        LogicalSize {
            w: width,
            h: height,
        },
    )
}
