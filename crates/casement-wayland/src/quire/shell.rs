//! `quire_shell_v1`: the shell's surface roles (which layer surface is the bar, the dock, the
//! switcher), the minimise targets the dock gives its tiles, and the events only the
//! compositor sees (the real modifier keys, edge reveals). Shell connections
//! only. Frozen at P2-Z; filled by P2-Q2.
//!
//! The wire glue is written: the part, the handler trait, the global's and the object's
//! dispatch, and the errors a refusal becomes. What each request decides (`on_*`) and what a
//! note sends (`apply_note`) are the lane's. The switcher and show-desktop requests are ignored
//! until P3-Z gives them their machines.

use super::code_of;
use super::shell_permission::{self, SentAsk};
use super::space_shot::SpaceShotPart;
use super::{shell_overview, shell_screen, shell_space, shell_split, shell_trial};
use crate::foreign_toplevel_list::{identifier_of, window_of_identifier};
use crate::handler::{EventSink, OutputLookup};
use crate::states::{GlobalFilter, WaylandFault};
use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize};
use casement_core::overview_drag::SpaceTargets;
use casement_core::policy::GlobalKind;
use casement_core::privilege::UnixSeconds;
use casement_core::shell::{ShellNote, ShellRequest};
use casement_core::time::Mono;
use casement_core::{Event, LayerKey, OutputKey, WindowId};
use casement_protocols::server::quire_shell_v1::quire_shell_v1::{self, QuireShellV1};
use casement_protocols::vocab::{MacMods, ShellRole, WindowEntry, WireCode};
use porter_core::SpaceId;
use smithay::reexports::wayland_server::backend::{ClientId, GlobalId};
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::{
    Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch, New, Resource,
};
use smithay::wayland::image_capture_source::ImageCaptureSourceHandler;
use smithay::wayland::{Dispatch2, GlobalDispatch2};
use std::collections::BTreeMap;

/// The version casement serves (7: the bar in full screen, reduced motion, `reveal_on_output`).
pub const VERSION: u32 = 7;

/// The surface that holds a role.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleHolder {
    /// The layer surface's key in the model.
    pub layer: LayerKey,
    /// The role it holds.
    pub role: ShellRole,
    /// The output it was made for (none when no output could be picked): a bar and a dock are
    /// held once per output.
    pub output: Option<OutputKey>,
    /// The surface (the target of the events addressed to the role).
    pub surface: WlSurface,
}

/// A role a layer surface asked for and the model has not yet answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AskedRole {
    /// The surface.
    pub surface: WlSurface,
    /// Its output.
    pub output: Option<OutputKey>,
}

/// `quire_shell_v1` and the objects bound on it.
#[derive(Debug)]
pub struct QuireShellPart {
    /// The global.
    pub global: GlobalId,
    /// Every bound `quire_shell_v1`.
    pub bound: Vec<QuireShellV1>,
    /// The surface holding a role, by layer (a layer holds one role at a time; a bar and a dock
    /// are held once per output, every other role once).
    pub roles: BTreeMap<LayerKey, RoleHolder>,
    /// The surface each layer asked a role from, until the model's answer (`ShellNote::Role`)
    /// says which one holds it: the note carries the layer, the wire knows the surface.
    pub asked: BTreeMap<LayerKey, AskedRole>,
    /// The modifier keys last sent, so a repeat of the same set sends nothing.
    pub mods: MacMods,
    /// The rectangles the shell set for dropping a window on a Space (version 2).
    pub space_targets: SpaceTargets,
    /// The Space pictures: sources, sessions and held frames (version 2).
    pub shots: SpaceShotPart,
    /// The consent questions sent to the shell and not yet settled (version 5).
    pub asks: Vec<SentAsk>,
    /// The display trial the shell was asked about and has not seen end (version 6).
    pub trial: Option<shell_trial::OpenTrial>,
}

impl QuireShellPart {
    /// The part; creates the global (version 1) behind the I7 filter.
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> QuireShellPart
    where
        D: GlobalDispatch<QuireShellV1, ShellGlobal> + 'static,
    {
        let global = display.create_global::<D, QuireShellV1, _>(
            VERSION,
            ShellGlobal {
                filter: role_filter,
            },
        );
        QuireShellPart {
            global,
            bound: Vec::new(),
            roles: BTreeMap::new(),
            asked: BTreeMap::new(),
            mods: MacMods::NONE,
            space_targets: SpaceTargets::default(),
            shots: SpaceShotPart::default(),
            asks: Vec::new(),
            trial: None,
        }
    }
}

/// What the composition root provides to this protocol's dispatch.
pub trait QuireShellHandler:
    ImageCaptureSourceHandler + EventSink + OutputLookup + 'static
{
    /// The part.
    fn quire_shell_part(&mut self) -> &mut QuireShellPart;
    /// Now on the loop's clock (the seam: the wire reads no clock of its own).
    fn quire_shell_now(&self) -> Mono;
    /// The date in whole seconds (the stamp on a permission answer that is kept; core reads
    /// no clock).
    fn quire_shell_wall(&self) -> UnixSeconds;
    /// The layer surface `surface` is, when it is a live one of `client`.
    fn layer_of_surface(&self, client: &Client, surface: &WlSurface) -> Option<LayerKey>;
    /// The output `layer` was made for, or the one the compositor put it on.
    fn layer_output(&self, layer: LayerKey) -> Option<OutputKey>;
    /// Whether the model knows the window `window` (a live toplevel).
    fn window_known(&self, window: WindowId) -> bool;
    /// Whether the model has the Space `space` (version 2: a picture of one that is gone is a
    /// source whose sessions stop at once).
    fn space_known(&self, space: &SpaceId) -> bool;
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
        state.quire_shell_part().bound.push(shell.clone());
        let events = shell_permission::on_bind(state.quire_shell_part(), &shell);
        let now = state.quire_shell_now();
        shell_trial::on_bind(state.quire_shell_part(), &shell, now);
        state.feed(events);
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
        data_init: &mut DataInit<'_, D>,
    ) {
        match request {
            quire_shell_v1::Request::SetRole { surface, role } => {
                let Some(role) = code_of(role).and_then(|code| ShellRole::from_code(code).ok())
                else {
                    return;
                };
                let layer = state.layer_of_surface(client, &surface);
                let output = layer.and_then(|layer| state.layer_output(layer));
                match on_set_role(state.quire_shell_part(), layer, output, &surface, role) {
                    Ok(events) => state.feed(events),
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
                let window = window_of_identifier(&toplevel).filter(|id| state.window_known(*id));
                match on_set_minimise_target(state.quire_shell_part(), dock, window, local) {
                    Ok(events) => state.feed(events),
                    Err(refusal) => refuse_target(resource, refusal),
                }
            }
            quire_shell_v1::Request::ClearMinimiseTarget { toplevel } => {
                match on_clear_minimise_target(state.quire_shell_part(), &toplevel) {
                    Ok(events) => state.feed(events),
                    Err(refusal) => refuse_target(resource, refusal),
                }
            }
            quire_shell_v1::Request::WindowMenuPick { toplevel, entry } => {
                let window = window_of_identifier(&toplevel).filter(|id| state.window_known(*id));
                let entry = code_of(entry).and_then(|code| WindowEntry::from_code(code).ok());
                state.feed(on_window_menu_pick(window, entry));
            }
            quire_shell_v1::Request::SplitPick { toplevel } => {
                let window = window_of_identifier(&toplevel).filter(|id| state.window_known(*id));
                state.feed(shell_split::on_split_pick(window));
            }
            quire_shell_v1::Request::SplitCancel => {
                state.feed(shell_split::on_split_cancel());
            }
            quire_shell_v1::Request::SplitDivide { toplevel, share } => {
                let window = window_of_identifier(&toplevel).filter(|id| state.window_known(*id));
                state.feed(shell_split::on_split_divide(window, share));
            }
            quire_shell_v1::Request::Overview { mode, toggle } => {
                state.feed(shell_overview::on_overview(code_of(mode), code_of(toggle)));
            }
            quire_shell_v1::Request::PermissionAnswer { ask, answer } => {
                let now = state.quire_shell_now();
                let wall = state.quire_shell_wall();
                match shell_permission::on_answer(state.quire_shell_part(), ask, answer, now, wall)
                {
                    Ok(events) => state.feed(events),
                    Err(fault) => resource
                        .post_error(quire_shell_v1::Error::AnsweredTooSoon, fault.to_string()),
                }
            }
            quire_shell_v1::Request::DisplaysTrialAnswer { trial, answer } => {
                let now = state.quire_shell_now();
                match shell_trial::on_answer(state.quire_shell_part(), trial, answer, now) {
                    Ok(events) => state.feed(events),
                    Err(fault) => resource
                        .post_error(quire_shell_v1::Error::AnsweredTooSoon, fault.to_string()),
                }
            }
            quire_shell_v1::Request::SetFullScreenBar { mode } => {
                state.feed(shell_screen::on_set_full_screen_bar(code_of(mode)));
            }
            quire_shell_v1::Request::SetMotion { motion } => {
                state.feed(shell_screen::on_set_motion(code_of(motion)));
            }
            // The switcher and show-desktop requests arrive with P3-Z; the version 2 requests
            // are `shell_space`'s, which ignores the rest.
            other => shell_space::on_request(state, resource, other, data_init),
        }
    }

    fn destroyed(&self, state: &mut D, _client: ClientId, resource: &QuireShellV1) {
        on_shell_gone(state.quire_shell_part(), resource);
        let events = shell_permission::on_unbind(state.quire_shell_part(), resource.version());
        state.feed(events);
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

/// `set_role`: `surface` (the layer `layer` made for `output`, when it is one of the client's)
/// takes `role`. Refuses a surface that is no layer surface of the client and a role another live
/// surface holds (for a bar or a dock, another on the same output); otherwise the request the
/// model takes (`Event::Shell(ShellRequest::Role)`).
pub fn on_set_role(
    part: &mut QuireShellPart,
    layer: Option<LayerKey>,
    output: Option<OutputKey>,
    surface: &WlSurface,
    role: ShellRole,
) -> Result<Vec<Event>, RoleRefusal> {
    let layer = layer.ok_or(RoleRefusal::NotALayer)?;
    let taken = part.roles.values().any(|holder| {
        holder.layer != layer && holder.role == role && shares(role, holder, output.as_ref())
    });
    if taken {
        return Err(RoleRefusal::Taken);
    }
    part.asked.insert(
        layer,
        AskedRole {
            surface: surface.clone(),
            output,
        },
    );
    Ok(vec![Event::Shell(ShellRequest::Role { layer, role })])
}

/// Whether `holder` stands in the way of a second surface on `output` taking `role`: a bar or a
/// dock only on the same output, any other role anywhere.
fn shares(role: ShellRole, holder: &RoleHolder, output: Option<&OutputKey>) -> bool {
    match role {
        ShellRole::Bar | ShellRole::Dock => holder.output.as_ref() == output,
        _ => true,
    }
}

/// `set_minimise_target`: where the window `window` (the `ext_foreign_toplevel_list`
/// identifier the client sent, turned into the model's window when the model knows it; `None`
/// when it is no identifier or names no live window) shrinks into, `local` to the `dock`
/// layer surface.
pub fn on_set_minimise_target(
    _part: &mut QuireShellPart,
    dock: Option<LayerKey>,
    window: Option<WindowId>,
    local: LogicalRect,
) -> Result<Vec<Event>, TargetRefusal> {
    let dock = dock.ok_or(TargetRefusal::NotALayer)?;
    let window = window.ok_or(TargetRefusal::UnknownToplevel)?;
    Ok(vec![Event::Shell(ShellRequest::MinimiseTarget {
        dock,
        window,
        local,
    })])
}

/// `clear_minimise_target`: forget the target of the window `toplevel`.
pub fn on_clear_minimise_target(
    _part: &mut QuireShellPart,
    toplevel: &str,
) -> Result<Vec<Event>, TargetRefusal> {
    let window = window_of_identifier(toplevel).ok_or(TargetRefusal::UnknownToplevel)?;
    Ok(vec![Event::Shell(ShellRequest::ClearMinimiseTarget {
        window,
    })])
}

/// `window_menu_pick`: the entry `entry` of the menu of `window`. A window the model does not
/// know and an entry the XML does not name (both `None`) are ignored, with no error: the menu
/// was on screen while the window went away.
///
/// ```
/// use casement_core::WindowId;
/// use casement_protocols::vocab::WindowEntry;
/// use casement_wayland::quire::shell::on_window_menu_pick;
/// assert_eq!(on_window_menu_pick(Some(WindowId(1)), Some(WindowEntry::Left)).len(), 1);
/// assert!(on_window_menu_pick(None, Some(WindowEntry::Left)).is_empty());
/// assert!(on_window_menu_pick(Some(WindowId(1)), None).is_empty());
/// ```
pub fn on_window_menu_pick(window: Option<WindowId>, entry: Option<WindowEntry>) -> Vec<Event> {
    window
        .zip(entry)
        .map(|(window, entry)| Event::Shell(ShellRequest::WindowMenuPick { window, entry }))
        .into_iter()
        .collect()
}

/// A bound object went away: the roles it left are not forgotten (the surfaces stay until
/// they unmap), only the object is.
pub fn on_shell_gone(part: &mut QuireShellPart, shell: &QuireShellV1) {
    part.bound.retain(|bound| bound != shell);
    shell_space::forget_targets(part);
}

/// A layer surface unmapped: whatever role it held is free again.
pub fn on_layer_gone(part: &mut QuireShellPart, layer: LayerKey) {
    part.roles.remove(&layer);
    part.asked.remove(&layer);
}

/// The model said `note`: record it (a role taken, a minimise target) or send it to the
/// shell (`modifiers` to the role surfaces; a reveal is sent by [`shell_screen::send_reveal`], which
/// needs the outputs).
pub fn apply_note(part: &mut QuireShellPart, note: &ShellNote) -> Result<(), WaylandFault> {
    match note {
        ShellNote::Role(layer, role) => {
            if let Some(AskedRole { surface, output }) = part.asked.remove(layer) {
                // A layer holds one role at a time: the one it had before is free.
                part.roles.insert(
                    *layer,
                    RoleHolder {
                        layer: *layer,
                        role: *role,
                        output,
                        surface,
                    },
                );
            }
        }
        // The table lives in the model; the dock bounce that would use it is P3's.
        ShellNote::MinimiseTarget(..) => {}
        ShellNote::Modifiers(mods) => {
            if part.mods != *mods {
                part.mods = *mods;
                send_modifiers(part, *mods);
            }
        }
        ShellNote::WindowMenu(show) => {
            let identifier = identifier_of(show.window);
            for shell in &part.bound {
                shell.window_menu(identifier.clone(), show.at.x, show.at.y, show.enabled());
            }
        }
        ShellNote::SplitChoose(choices) => shell_split::send_choose(part, choices),
        ShellNote::SplitChoiceEnd(first) => shell_split::send_choice_end(part, *first),
        ShellNote::Overview(overview) => shell_overview::send_state(part, *overview),
        _ => {}
    }
    Ok(())
}

/// `modifiers(surface, mods)` to the surface of every held role, on every bound object of the
/// same client, whether or not the surface has keyboard focus.
fn send_modifiers(part: &QuireShellPart, mods: MacMods) {
    let wire = quire_shell_v1::MacMods::from_bits_truncate(mods.0);
    for shell in &part.bound {
        for holder in part.roles.values() {
            if holder.surface.client() == shell.client() {
                shell.modifiers(&holder.surface, wire);
            }
        }
    }
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
