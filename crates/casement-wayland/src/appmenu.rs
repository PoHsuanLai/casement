//! `org_kde_kwin_appmenu_manager` / `org_kde_kwin_appmenu`: a Qt client tells the compositor
//! the D-Bus service and object path of its `com.canonical.dbusmenu` (P3-MN). casement never
//! reads the menu; it records the address on the window (`WindowHintChange::Menu`) and the
//! shell hears it through `quire_desktop_app_v1.menu_source`.
//!
//! This file is the meaning of the requests as pure functions. The global itself needs the
//! protocol's generated server code, which no crate in the workspace carries yet: the
//! dependency ask is in FINDINGS ("Global menus"). The wire glue (the global, the dispatch of
//! `create`, `set_address` and `release`) calls these.
//!
//! A window's address is staged until the window maps: the client may declare it on the
//! `wl_surface` before the first buffer, and the model only knows mapped windows.

use casement_core::app::MenuSource;
use casement_core::{Event, WindowHintChange, WindowId};
use std::collections::BTreeMap;

/// The longest D-Bus name or object path (the D-Bus specification's 255 bytes).
pub const NAME_MAX_LEN: usize = 255;

/// What the appmenu objects declared, per window.
#[derive(Debug, Default)]
pub struct AppMenuPart {
    /// The address each window declared, kept for the map and for a later change.
    pub declared: BTreeMap<WindowId, MenuSource>,
}

impl AppMenuPart {
    /// An empty table.
    pub fn new() -> AppMenuPart {
        AppMenuPart::default()
    }
}

/// `set_address`: the window's menu is at service `service`, object `path`. An address the
/// D-Bus specification does not allow is ignored (the previous one stands). `mapped` says
/// whether the model already has the window.
pub fn on_address(
    part: &mut AppMenuPart,
    window: WindowId,
    service: &str,
    path: &str,
    mapped: Mapped,
) -> Vec<Event> {
    if !valid_bus_name(service) || !valid_object_path(path) {
        return Vec::new();
    }
    let source = MenuSource::DbusMenu {
        bus: service.to_owned(),
        path: path.to_owned(),
    };
    part.declared.insert(window, source.clone());
    announce(window, source, mapped)
}

/// `release` (or the surface's death): the window has no global menu any more.
pub fn on_released(part: &mut AppMenuPart, window: WindowId, mapped: Mapped) -> Vec<Event> {
    match part.declared.remove(&window) {
        Some(_) => announce(window, MenuSource::None, mapped),
        None => Vec::new(),
    }
}

/// The window mapped: what it declared before goes to the model.
pub fn on_mapped(part: &AppMenuPart, window: WindowId) -> Vec<Event> {
    part.declared
        .get(&window)
        .map(|source| announce(window, source.clone(), Mapped::Yes))
        .unwrap_or_default()
}

/// The window is gone: its address is forgotten (the model drops the window with it).
pub fn on_forgotten(part: &mut AppMenuPart, window: WindowId) {
    part.declared.remove(&window);
}

/// Whether the model holds the window yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mapped {
    /// It does: the hint can go now.
    Yes,
    /// Not yet: the table keeps the address until [`on_mapped`].
    No,
}

fn announce(window: WindowId, source: MenuSource, mapped: Mapped) -> Vec<Event> {
    match mapped {
        Mapped::Yes => vec![Event::WindowHint {
            window,
            hint: WindowHintChange::Menu(source),
        }],
        Mapped::No => Vec::new(),
    }
}

/// Whether `name` is a well-formed D-Bus bus name: a unique name (`:1.42`) or a well-known one
/// (`org.kde.Foo`), at most 255 bytes, two or more elements of letters, digits, `_` and `-`.
/// Elements of a well-known name do not start with a digit.
pub fn valid_bus_name(name: &str) -> bool {
    let (unique, rest) = match name.strip_prefix(':') {
        Some(rest) => (true, rest),
        None => (false, name),
    };
    let element = |part: &str| {
        !part.is_empty()
            && part
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
            && (unique || !part.as_bytes()[0].is_ascii_digit())
    };
    name.len() <= NAME_MAX_LEN && rest.split('.').count() >= 2 && rest.split('.').all(element)
}

/// Whether `path` is a well-formed D-Bus object path: `/`, or `/` followed by non-empty
/// elements of letters, digits and `_` separated by `/`, with no trailing `/`.
pub fn valid_object_path(path: &str) -> bool {
    path.len() <= NAME_MAX_LEN
        && path.starts_with('/')
        && (path == "/"
            || path[1..].split('/').all(|part| {
                !part.is_empty() && part.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
            }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bus_names_and_paths_follow_the_dbus_rules() {
        let names = [
            ("unique", ":1.42", true),
            ("well known", "org.kde.Foo", true),
            ("dash", "org.kde.foo-bar", true),
            ("one element", "Foo", false),
            ("digit start", "org.1foo", false),
            ("empty element", "org..foo", false),
            ("space", "org.kde foo", false),
            ("empty", "", false),
        ];
        for (name, text, want) in names {
            assert_eq!(valid_bus_name(text), want, "name {name}");
        }
        let paths = [
            ("root", "/", true),
            ("deep", "/MenuBar/1", true),
            ("no slash", "MenuBar", false),
            ("trailing", "/MenuBar/", false),
            ("double", "//x", false),
            ("dot", "/a.b", false),
            ("empty", "", false),
        ];
        for (name, text, want) in paths {
            assert_eq!(valid_object_path(text), want, "path {name}");
        }
    }

    #[test]
    fn an_address_waits_for_the_map_and_a_bad_one_is_ignored() {
        let mut part = AppMenuPart::new();
        let window = WindowId(3);
        assert!(on_address(&mut part, window, "org.kde.Foo", "/Menu", Mapped::No).is_empty());
        assert!(on_address(&mut part, window, "bad", "/Menu", Mapped::No).is_empty());
        let want = Event::WindowHint {
            window,
            hint: WindowHintChange::Menu(MenuSource::DbusMenu {
                bus: "org.kde.Foo".into(),
                path: "/Menu".into(),
            }),
        };
        assert_eq!(on_mapped(&part, window), vec![want]);
        assert_eq!(
            on_released(&mut part, window, Mapped::Yes),
            vec![Event::WindowHint {
                window,
                hint: WindowHintChange::Menu(MenuSource::None)
            }]
        );
        assert!(on_mapped(&part, window).is_empty());
    }
}
