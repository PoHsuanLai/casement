//! `gtk_shell1` (only `gtk_surface1.set_dbus_properties`, P3-MN): a GTK 3 client tells the
//! compositor where its `org.gtk.Menus` and `org.gtk.Actions` objects are. casement records
//! them on the window and the shell hears them through `quire_desktop_app_v1.menu_source`.
//! Every other request of the protocol is the glue's to accept and ignore.
//!
//! Pure functions only, like [`crate::appmenu`]; the generated server code is the blocked
//! dependency (FINDINGS, "Global menus").

use crate::appmenu::{AppMenuPart, Mapped, valid_bus_name, valid_object_path};
use casement_core::app::MenuSource;
use casement_core::{Event, WindowHintChange, WindowId};

/// What `set_dbus_properties` carried; every argument may be absent (a null string).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DbusProperties {
    /// `application_id`, ignored (the app's key is proven by its process, never by this).
    pub application_id: Option<String>,
    /// `app_menu_path`, the GMenu of the application menu, ignored (macOS has no app menu in
    /// the window).
    pub app_menu_path: Option<String>,
    /// `menubar_path`.
    pub menubar_path: Option<String>,
    /// `window_object_path`.
    pub window_object_path: Option<String>,
    /// `application_object_path`.
    pub application_object_path: Option<String>,
    /// `unique_bus_name`.
    pub unique_bus_name: Option<String>,
}

/// `set_dbus_properties`: a client that names a bus and a menu bar path has a global menu. A
/// call without a menu bar path or with a malformed name or path clears the window's menu: GTK
/// calls it again with the new values whenever its menu model changes.
pub fn on_properties(
    part: &mut AppMenuPart,
    window: WindowId,
    props: &DbusProperties,
    mapped: Mapped,
) -> Vec<Event> {
    let source = source_of(props);
    match &source {
        MenuSource::None => {
            part.declared.remove(&window);
        }
        declared => {
            part.declared.insert(window, declared.clone());
        }
    }
    match mapped {
        Mapped::Yes => vec![Event::WindowHint {
            window,
            hint: WindowHintChange::Menu(source),
        }],
        Mapped::No => Vec::new(),
    }
}

/// The source the properties describe.
pub fn source_of(props: &DbusProperties) -> MenuSource {
    let optional = |value: &Option<String>| match value.as_deref() {
        None | Some("") => Some(String::new()),
        Some(path) => valid_object_path(path).then(|| path.to_owned()),
    };
    let bus = props
        .unique_bus_name
        .as_deref()
        .filter(|bus| valid_bus_name(bus));
    let bar = props
        .menubar_path
        .as_deref()
        .filter(|bar| !bar.is_empty() && valid_object_path(bar));
    match (
        bus,
        bar,
        optional(&props.application_object_path),
        optional(&props.window_object_path),
    ) {
        (Some(bus), Some(bar), Some(app_path), Some(window_path)) => MenuSource::GtkMenus {
            bus: bus.to_owned(),
            app_path,
            menubar_path: bar.to_owned(),
            window_path,
        },
        _ => MenuSource::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn props(bus: Option<&str>, bar: Option<&str>) -> DbusProperties {
        DbusProperties {
            unique_bus_name: bus.map(str::to_owned),
            menubar_path: bar.map(str::to_owned),
            application_object_path: Some("/org/gtk/Application".into()),
            window_object_path: Some("/org/gtk/Window/1".into()),
            ..DbusProperties::default()
        }
    }

    #[test]
    fn a_bus_and_a_menu_bar_path_make_a_gtk_source() {
        let rows = [
            ("full", props(Some(":1.9"), Some("/org/gtk/Menus")), true),
            ("no bar", props(Some(":1.9"), None), false),
            ("empty bar", props(Some(":1.9"), Some("")), false),
            ("no bus", props(None, Some("/org/gtk/Menus")), false),
            ("bad bar", props(Some(":1.9"), Some("org/gtk")), false),
        ];
        for (name, given, declared) in rows {
            assert_eq!(source_of(&given).is_declared(), declared, "{name}");
        }
        assert_eq!(
            source_of(&props(Some(":1.9"), Some("/org/gtk/Menus"))),
            MenuSource::GtkMenus {
                bus: ":1.9".into(),
                app_path: "/org/gtk/Application".into(),
                menubar_path: "/org/gtk/Menus".into(),
                window_path: "/org/gtk/Window/1".into(),
            }
        );
    }

    #[test]
    fn a_later_call_without_a_bar_clears_the_window() {
        let mut part = AppMenuPart::new();
        let window = WindowId(2);
        let set = on_properties(
            &mut part,
            window,
            &props(Some(":1.9"), Some("/org/gtk/Menus")),
            Mapped::Yes,
        );
        assert_eq!(set.len(), 1);
        assert!(part.declared.contains_key(&window));
        let cleared = on_properties(&mut part, window, &props(Some(":1.9"), None), Mapped::Yes);
        assert_eq!(
            cleared,
            vec![Event::WindowHint {
                window,
                hint: WindowHintChange::Menu(MenuSource::None)
            }]
        );
        assert!(part.declared.is_empty());
    }
}
