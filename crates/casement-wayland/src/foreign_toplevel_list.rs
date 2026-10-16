//! `ext_foreign_toplevel_list_v1`: every window, for docks, switchers and taskbars outside
//! the shell. Hidden from sandboxed clients (I7). Identifiers are chosen by casement.

use crate::states::{GlobalFilter, WaylandFault};
use casement_core::WindowId;
use casement_core::lock::LockState;
use casement_core::policy::GlobalKind;
use casement_core::window::Window;
use smithay::reexports::wayland_protocols::ext::foreign_toplevel_list::v1::server::{
    ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1,
    ext_foreign_toplevel_list_v1::ExtForeignToplevelListV1,
};
use smithay::reexports::wayland_server::{Dispatch, DisplayHandle, GlobalDispatch};
use smithay::wayland::foreign_toplevel_list::{
    ForeignToplevelHandle, ForeignToplevelListGlobalData, ForeignToplevelListHandler,
    ForeignToplevelListState,
};
use std::collections::BTreeMap;

/// Makes a handle for a window: title, app id, identifier. Built in [`ForeignToplevelListPart::new`]
/// where the state type is known, so [`apply_toplevel`] needs no type parameter.
type Spawn = fn(&mut ForeignToplevelListState, String, String, String) -> ForeignToplevelHandle;

/// `ext_foreign_toplevel_list_v1` and the handle of each listed window.
#[derive(Debug)]
pub struct ForeignToplevelListPart {
    /// smithay's state.
    pub state: ForeignToplevelListState,
    /// Each listed window's handle.
    pub handles: BTreeMap<WindowId, ForeignToplevelHandle>,
    spawn: Spawn,
}

impl ForeignToplevelListPart {
    /// Creates the global (version 1), filtered by the I7 table.
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> ForeignToplevelListPart
    where
        D: ForeignToplevelListHandler
            + GlobalDispatch<ExtForeignToplevelListV1, ForeignToplevelListGlobalData>
            + Dispatch<ExtForeignToplevelHandleV1, ForeignToplevelHandle>,
    {
        ForeignToplevelListPart {
            state: ForeignToplevelListState::new_with_filter::<D>(
                display,
                role_filter.for_global(GlobalKind::ForeignToplevelList),
            ),
            handles: BTreeMap::new(),
            spawn: |state, title, app_id, identifier| {
                state.new_toplevel_with_identifier::<D>(title, app_id, identifier)
            },
        }
    }
}

/// The identifier clients see for `window`: casement's own, stable for the window's life and
/// never reused in a session (window ids are not), printable ASCII of at most 32 bytes.
pub fn identifier_of(window: WindowId) -> String {
    format!("w{}", window.0)
}

/// The window an identifier names: the inverse of [`identifier_of`] (`None` for text casement
/// did not make). The desktop and shell protocols name windows by it across connections.
pub fn window_of_identifier(identifier: &str) -> Option<WindowId> {
    let digits = identifier.strip_prefix('w')?;
    let plain = !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit());
    plain.then(|| digits.parse().ok().map(WindowId)).flatten()
}

/// How much of a window a list client is told. A locked session tells nothing of what is
/// open: titles and app ids are the person's mail, documents and sites, and the window-list
/// protocols are open to every client that binds them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disclosure {
    /// The title and the app id.
    Shown,
    /// Neither: both go out empty.
    Withheld,
}

impl Disclosure {
    /// What the session being in `lock` lets a list client be told.
    pub fn of(lock: &LockState) -> Disclosure {
        match lock {
            LockState::Unlocked => Disclosure::Shown,
            LockState::Locking { .. }
            | LockState::Locked { .. }
            | LockState::LockedWithoutSurface => Disclosure::Withheld,
        }
    }

    /// `text` as a list client is told it.
    pub fn text(self, text: &str) -> &str {
        match self {
            Disclosure::Shown => text,
            Disclosure::Withheld => "",
        }
    }
}

/// Lists `window` (or sends its changed title and app id; nothing is sent when neither
/// changed). A title the protocol cannot carry (the text is app-authored) goes out as given;
/// both are empty when `disclosure` withholds them.
pub fn apply_toplevel(
    part: &mut ForeignToplevelListPart,
    window: &Window,
    disclosure: Disclosure,
) -> Result<(), WaylandFault> {
    let title = disclosure.text(window.title.0.as_str());
    let app_id = disclosure.text(window.claimed_id.as_str());
    match part.handles.get(&window.id) {
        Some(handle) => {
            let changed = handle.title() != title || handle.app_id() != app_id;
            handle.send_title(title);
            handle.send_app_id(app_id);
            if changed {
                handle.send_done();
            }
        }
        None => {
            let handle = (part.spawn)(
                &mut part.state,
                title.to_owned(),
                app_id.to_owned(),
                identifier_of(window.id),
            );
            part.handles.insert(window.id, handle);
        }
    }
    Ok(())
}

/// Withdraws `window`: its handles get `closed` and the window is forgotten.
pub fn apply_closed(
    part: &mut ForeignToplevelListPart,
    window: WindowId,
) -> Result<(), WaylandFault> {
    let handle = part
        .handles
        .remove(&window)
        .ok_or(WaylandFault::UnknownWindow(window))?;
    part.state.remove_toplevel(&handle);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_locked_session_withholds_what_is_open() {
        let locking = LockState::Locking {
            since: casement_core::time::Mono(std::time::Duration::ZERO),
        };
        for (lock, want) in [
            (LockState::Unlocked, "Inbox"),
            (locking, ""),
            (LockState::LockedWithoutSurface, ""),
        ] {
            assert_eq!(Disclosure::of(&lock).text("Inbox"), want, "{lock:?}");
        }
    }

    #[test]
    fn identifiers_are_stable_distinct_and_fit_the_protocol() {
        let (a, b) = (identifier_of(WindowId(7)), identifier_of(WindowId(8)));
        assert_eq!(a, identifier_of(WindowId(7)));
        assert_ne!(a, b);
        let longest = identifier_of(WindowId(u64::MAX));
        assert!(longest.len() <= 32 && longest.is_ascii());
    }

    #[test]
    fn an_identifier_names_its_window_and_only_its_own() {
        for id in [0, 7, u64::MAX] {
            assert_eq!(
                window_of_identifier(&identifier_of(WindowId(id))),
                Some(WindowId(id))
            );
        }
        for text in [
            "",
            "w",
            "7",
            "x7",
            "w-7",
            "w7 ",
            "w+7",
            "w99999999999999999999",
        ] {
            assert_eq!(window_of_identifier(text), None, "{text:?}");
        }
    }
}
