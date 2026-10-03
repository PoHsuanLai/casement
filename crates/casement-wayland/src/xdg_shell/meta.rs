//! The metadata requests of a toplevel: title, app id, parent and limits. Each reports a change
//! to the model once, and only for a mapped window (the map event carries what was declared
//! before it).

use super::XdgShellPart;
use super::commit::{Declared, Phase, ToplevelRecord, limits_change, parent_of};
use super::declared::{claimed_of, role_of};
use casement_core::{Event, ToplevelMeta, WindowId};
use casement_protocols::vocab::WindowRole;
use smithay::wayland::shell::xdg::ToplevelSurface;

/// The window and its record when `surface` is a mapped toplevel.
fn mapped<'a>(
    part: &'a mut XdgShellPart,
    surface: &ToplevelSurface,
) -> Option<(WindowId, &'a mut ToplevelRecord)> {
    let window = part.window_of_surface(surface.wl_surface())?;
    part.records
        .get_mut(&window)
        .filter(|record| matches!(record.phase, Phase::Mapped(_)))
        .map(|record| (window, record))
}

/// The title changed.
pub fn on_title(part: &mut XdgShellPart, surface: &ToplevelSurface) -> Vec<Event> {
    let title = Declared::read(surface).title;
    let Some((window, record)) = mapped(part, surface) else {
        return Vec::new();
    };
    if record.title == title {
        return Vec::new();
    }
    record.title = title.clone();
    vec![Event::ToplevelMeta {
        window,
        meta: ToplevelMeta::Title(title),
    }]
}

/// The app id changed.
pub fn on_app_id(part: &mut XdgShellPart, surface: &ToplevelSurface) -> Vec<Event> {
    let declared = Declared::read(surface);
    let Some((window, record)) = mapped(part, surface) else {
        return Vec::new();
    };
    let claimed = claimed_of(declared.app_id.as_deref(), record.client);
    if record.claimed == claimed {
        return Vec::new();
    }
    record.claimed = claimed.clone();
    vec![Event::ToplevelMeta {
        window,
        meta: ToplevelMeta::ClaimedId(claimed),
    }]
}

/// The parent changed (an `xdg_foreign` import counts).
pub fn on_parent(part: &mut XdgShellPart, surface: &ToplevelSurface) -> Vec<Event> {
    let parent = surface.parent();
    let resolved = parent_of(parent.as_ref(), &part.toplevels);
    let Some((window, record)) = mapped(part, surface) else {
        return Vec::new();
    };
    if record.parent == resolved {
        return Vec::new();
    }
    record.parent = resolved;
    vec![Event::ToplevelMeta {
        window,
        meta: ToplevelMeta::Parent(resolved),
    }]
}

/// The size limits changed (read on commit).
pub fn on_limits(part: &mut XdgShellPart, surface: &ToplevelSurface) -> Vec<Event> {
    let limits = Declared::read(surface).limits;
    let Some((window, record)) = mapped(part, surface) else {
        return Vec::new();
    };
    limits_change(window, record, limits).into_iter().collect()
}

/// The role `window`'s toplevel has from the protocols that name one (xdg-dialog), without any
/// `quire_window_v1.set_role`.
pub fn base_role(part: &XdgShellPart, window: WindowId) -> WindowRole {
    part.toplevels
        .get(&window)
        .map_or(WindowRole::Document, |toplevel| {
            role_of(Declared::read(toplevel).hint)
        })
}
