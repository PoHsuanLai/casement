//! What a toplevel's commit means. The first commit (no buffer) gets the initial configure; the
//! first commit with a buffer after it maps the window; later commits resize it, change its
//! limits, or (with a null buffer) unmap it. The decision is a pure table ([`commit_step`]);
//! reading smithay's surface state and building events are the two edges around it.
//!
//! The commit handler runs smithay's buffer bookkeeping (`on_commit_buffer_handler`) before it
//! routes here, so the buffer's size is read from the renderer surface state.

use super::XdgShellPart;
use super::declared::{claimed_of, limits_of, modality_of, role_of};
use crate::client::ClientState;
use crate::convert::size_of;
use casement_core::app::AppKey;
use casement_core::focus::TokenCheck;
use casement_core::geometry::LogicalSize;
use casement_core::window::{SizeLimits, WindowTitle};
use casement_core::{ClientKey, Event, MappedToplevel, ToplevelMeta, ToplevelRequest, WindowId};
use porter_core::ClaimedId;
use smithay::backend::renderer::utils::with_renderer_surface_state;
use smithay::reexports::wayland_server::Resource;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::wayland::compositor::with_states;
use smithay::wayland::shell::xdg::dialog::ToplevelDialogHint;
use smithay::wayland::shell::xdg::{SurfaceCachedState, ToplevelSurface, XdgToplevelSurfaceData};

/// Where a toplevel is in its life.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Created, or unmapped again: no buffer is shown.
    Unmapped,
    /// Mapped, last committed at this size.
    Mapped(LogicalSize),
}

/// What the edge does about a commit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitStep {
    /// Send the initial configure.
    InitialConfigure,
    /// The window maps at this size.
    Map(LogicalSize),
    /// The mapped window now has this size.
    Resize(LogicalSize),
    /// The mapped window lost its buffer.
    Unmap,
    /// Nothing changed that the model needs to know.
    Nothing,
}

/// The step a commit takes: the whole life of a toplevel, one row per case.
/// `configured`: whether the initial configure has gone out. `buffer`: the logical size of the
/// buffer a commit left attached (the window geometry when the client set one), none when nothing
/// is attached.
pub fn commit_step(phase: Phase, configured: bool, buffer: Option<LogicalSize>) -> CommitStep {
    match (configured, phase, buffer) {
        // smithay resets a toplevel's role (the initial configure included) in the very commit
        // that removes its buffer, so a mapped window reads as not configured exactly then: it is
        // the unmap, and the configure follows the client's next bare commit.
        (false, Phase::Mapped(_), None) => CommitStep::Unmap,
        (false, _, _) => CommitStep::InitialConfigure,
        (true, Phase::Unmapped, Some(size)) => CommitStep::Map(size),
        (true, Phase::Unmapped, None) => CommitStep::Nothing,
        (true, Phase::Mapped(had), Some(size)) if had == size => CommitStep::Nothing,
        (true, Phase::Mapped(_), Some(size)) => CommitStep::Resize(size),
        (true, Phase::Mapped(_), None) => CommitStep::Unmap,
    }
}

/// What the edge remembers of one toplevel between commits, so a change is reported once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToplevelRecord {
    /// The client that owns it.
    pub client: ClientKey,
    /// Where it is in its life.
    pub phase: Phase,
    /// The limits the model last heard.
    pub limits: SizeLimits,
    /// The title the model last heard.
    pub title: WindowTitle,
    /// The app id the model last heard.
    pub claimed: ClaimedId,
    /// The parent the model last heard.
    pub parent: Option<WindowId>,
    /// How the activation token it asked with before mapping checked out (A7).
    pub staged: TokenCheck,
    /// What it asked to open as (maximised, full screen) before it mapped: the model hears it
    /// right after the map, and the initial configure already carried it.
    pub opening: Option<ToplevelRequest>,
}

/// What a client has declared about a toplevel, read from smithay's surface state.
#[derive(Debug, Clone)]
pub(super) struct Declared {
    pub(super) title: WindowTitle,
    pub(super) app_id: Option<String>,
    pub(super) parent: Option<WlSurface>,
    pub(super) hint: ToplevelDialogHint,
    pub(super) limits: SizeLimits,
    /// Whether the initial configure has gone out.
    pub(super) configured: bool,
    /// The size of the attached buffer, none when nothing is attached.
    pub(super) buffer: Option<LogicalSize>,
}

impl Declared {
    /// Reads `surface`'s role attributes, committed size limits and buffer.
    pub(super) fn read(surface: &ToplevelSurface) -> Declared {
        let wl = surface.wl_surface();
        let (title, app_id, hint, configured, limits, geometry) = with_states(wl, |states| {
            let role = states
                .data_map
                .get::<XdgToplevelSurfaceData>()
                .and_then(|data| data.lock().ok());
            let mut cached = states.cached_state.get::<SurfaceCachedState>();
            let current = cached.current();
            let limits = limits_of(
                (current.min_size.w, current.min_size.h),
                (current.max_size.w, current.max_size.h),
            );
            let geometry = current.geometry.map(|rect| size_of(rect.size));
            match role {
                Some(role) => (
                    WindowTitle(role.title.clone().unwrap_or_default()),
                    role.app_id.clone(),
                    role.dialog_hint,
                    role.initial_configure_sent,
                    limits,
                    geometry,
                ),
                None => (
                    WindowTitle::default(),
                    None,
                    ToplevelDialogHint::Unknown,
                    false,
                    limits,
                    geometry,
                ),
            }
        });
        let surface_size = with_renderer_surface_state(wl, |state| state.surface_size()).flatten();
        let buffer = match (surface_size, geometry) {
            (None, _) => None,
            (Some(_), Some(geometry)) => Some(geometry),
            (Some(size), None) => Some(size_of(size)),
        };
        Declared {
            title,
            app_id,
            parent: surface.parent(),
            hint,
            limits,
            configured,
            buffer,
        }
    }
}

/// A toplevel was created and given the id `window` (no model event until it maps).
pub fn on_new_toplevel(
    part: &mut XdgShellPart,
    surface: &ToplevelSurface,
    window: WindowId,
) -> Vec<Event> {
    let Some(client) = surface.wl_surface().client() else {
        return Vec::new();
    };
    let Some(state) = ClientState::of(&client) else {
        return Vec::new();
    };
    part.clients.insert(state.key, surface.client());
    part.toplevels.insert(window, surface.clone());
    part.records.insert(
        window,
        ToplevelRecord {
            client: state.key,
            phase: Phase::Unmapped,
            limits: limits_of((0, 0), (0, 0)),
            title: WindowTitle::default(),
            claimed: claimed_of(None, state.key),
            parent: None,
            staged: TokenCheck::Missing,
            opening: None,
        },
    );
    Vec::new()
}

/// A toplevel's surface committed: its initial configure, its mapping
/// (`Event::ToplevelMapped`, with the app `client` proves), or a new size.
pub fn on_toplevel_commit(
    part: &mut XdgShellPart,
    surface: &WlSurface,
    client: &ClientState,
) -> Vec<Event> {
    let Some(window) = part.window_of_surface(surface) else {
        return Vec::new();
    };
    let Some(toplevel) = part.toplevels.get(&window).cloned() else {
        return Vec::new();
    };
    let declared = Declared::read(&toplevel);
    let Some(record) = part.records.get_mut(&window) else {
        return Vec::new();
    };
    match commit_step(record.phase, declared.configured, declared.buffer) {
        CommitStep::InitialConfigure => {
            toplevel.send_configure();
            Vec::new()
        }
        CommitStep::Map(size) => {
            let events = mapped_events(window, client, &declared, size, record, &part.toplevels);
            record.phase = Phase::Mapped(size);
            events
        }
        CommitStep::Resize(size) => {
            record.phase = Phase::Mapped(size);
            let mut events = vec![Event::ToplevelCommitted { window, size }];
            events.extend(limits_change(window, record, declared.limits));
            events
        }
        CommitStep::Unmap => {
            record.phase = Phase::Unmapped;
            record.staged = TokenCheck::Missing;
            vec![Event::ToplevelUnmapped { window }]
        }
        CommitStep::Nothing => match record.phase {
            Phase::Mapped(_) => limits_change(window, record, declared.limits)
                .into_iter()
                .collect(),
            Phase::Unmapped => Vec::new(),
        },
    }
}

/// The toplevel was destroyed.
pub fn on_toplevel_destroyed(part: &mut XdgShellPart, surface: &ToplevelSurface) -> Vec<Event> {
    let Some(window) = part.window_of_surface(surface.wl_surface()) else {
        return Vec::new();
    };
    part.toplevels.remove(&window);
    match part.records.remove(&window) {
        Some(record) if matches!(record.phase, Phase::Mapped(_)) => {
            vec![Event::ToplevelUnmapped { window }]
        }
        _ => Vec::new(),
    }
}

/// The events a map is: `ToplevelMapped` (what the client declared before it mapped), then its
/// title. The record learns what the model was told.
pub(super) fn mapped_events(
    window: WindowId,
    client: &ClientState,
    declared: &Declared,
    first_size: LogicalSize,
    record: &mut ToplevelRecord,
    toplevels: &std::collections::BTreeMap<WindowId, ToplevelSurface>,
) -> Vec<Event> {
    let claimed = claimed_of(declared.app_id.as_deref(), client.key);
    let parent = parent_of(declared.parent.as_ref(), toplevels);
    let activation = std::mem::replace(&mut record.staged, TokenCheck::Missing);
    record.limits = declared.limits;
    record.title = declared.title.clone();
    record.claimed = claimed.clone();
    record.parent = parent;
    let mut events = vec![Event::ToplevelMapped(MappedToplevel {
        window,
        client: client.key,
        app: AppKey::of(&client.peer, &claimed),
        origin: client.origin,
        claimed_id: claimed,
        role: role_of(declared.hint),
        modality: modality_of(declared.hint),
        parent,
        limits: declared.limits,
        first_size,
        activation,
    })];
    if !declared.title.0.is_empty() {
        events.push(Event::ToplevelMeta {
            window,
            meta: ToplevelMeta::Title(declared.title.clone()),
        });
    }
    // What it asked to open as: the model hears it now that the window exists.
    events.extend(
        record
            .opening
            .take()
            .map(|request| Event::ToplevelRequest { window, request }),
    );
    events
}

/// The window `parent` is the root surface of, if it is one of ours.
pub(super) fn parent_of(
    parent: Option<&WlSurface>,
    toplevels: &std::collections::BTreeMap<WindowId, ToplevelSurface>,
) -> Option<WindowId> {
    parent
        .and_then(|surface| {
            toplevels
                .iter()
                .find(|(_, held)| held.wl_surface() == surface)
        })
        .map(|(id, _)| *id)
}

/// A `Limits` meta event when `limits` differ from what the model last heard.
pub(super) fn limits_change(
    window: WindowId,
    record: &mut ToplevelRecord,
    limits: SizeLimits,
) -> Option<Event> {
    (record.limits != limits).then(|| {
        record.limits = limits;
        Event::ToplevelMeta {
            window,
            meta: ToplevelMeta::Limits(limits),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn size(w: u32, h: u32) -> LogicalSize {
        LogicalSize { w, h }
    }

    /// Every case of a toplevel's life, one row each.
    #[test]
    fn the_commit_step_table() {
        use Phase::{Mapped, Unmapped};
        let rows = [
            (
                "first commit, no buffer: initial configure",
                Unmapped,
                false,
                None,
                CommitStep::InitialConfigure,
            ),
            (
                "a commit still unconfigured is configured first, even with a buffer",
                Unmapped,
                false,
                Some(size(10, 10)),
                CommitStep::InitialConfigure,
            ),
            (
                "configured, no buffer yet: nothing",
                Unmapped,
                true,
                None,
                CommitStep::Nothing,
            ),
            (
                "configured, first buffer: map",
                Unmapped,
                true,
                Some(size(640, 480)),
                CommitStep::Map(size(640, 480)),
            ),
            (
                "mapped, same size: nothing",
                Mapped(size(640, 480)),
                true,
                Some(size(640, 480)),
                CommitStep::Nothing,
            ),
            (
                "mapped, new size: resize",
                Mapped(size(640, 480)),
                true,
                Some(size(800, 600)),
                CommitStep::Resize(size(800, 600)),
            ),
            (
                "mapped, buffer removed: unmap",
                Mapped(size(640, 480)),
                true,
                None,
                CommitStep::Unmap,
            ),
            (
                "mapped, null buffer, smithay already reset the role: unmap",
                Mapped(size(640, 480)),
                false,
                None,
                CommitStep::Unmap,
            ),
            (
                "unmapped by a null buffer, smithay forgot the configure: configure again",
                Unmapped,
                false,
                None,
                CommitStep::InitialConfigure,
            ),
        ];
        for (case, phase, configured, buffer, want) in rows {
            assert_eq!(commit_step(phase, configured, buffer), want, "{case}");
        }
    }

    #[test]
    fn a_limits_change_is_reported_once() {
        let mut record = ToplevelRecord {
            client: ClientKey(1),
            phase: Phase::Mapped(size(1, 1)),
            limits: limits_of((0, 0), (0, 0)),
            title: WindowTitle::default(),
            claimed: claimed_of(None, ClientKey(1)),
            parent: None,
            staged: TokenCheck::Missing,
            opening: None,
        };
        let window = WindowId(3);
        let wider = limits_of((100, 100), (0, 0));
        assert_eq!(
            limits_change(window, &mut record, wider),
            Some(Event::ToplevelMeta {
                window,
                meta: ToplevelMeta::Limits(wider)
            })
        );
        assert_eq!(limits_change(window, &mut record, wider), None);
    }
}
