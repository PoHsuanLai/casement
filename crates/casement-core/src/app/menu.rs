//! Where an app's global menu lives, for apps that are not ours: a window's D-Bus menu address
//! as its client declared it (the KDE appmenu protocol for Qt, `gtk_shell1` for GTK 3). The
//! compositor never reads the menu; it records the address and tells the shell, which draws the
//! bar (03 §1.11).

use casement_protocols::vocab::MenuSourceKind;

/// A window's menu address.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum MenuSource {
    /// The window declared none.
    #[default]
    None,
    /// A `com.canonical.dbusmenu` object (Qt, through `org_kde_kwin_appmenu`).
    DbusMenu {
        /// The service's bus name.
        bus: String,
        /// The object path the menu is registered at.
        path: String,
    },
    /// `org.gtk.Menus` objects (GTK 3, through `gtk_shell1.set_dbus_properties`).
    GtkMenus {
        /// The client's unique bus name.
        bus: String,
        /// The application's object path (`org.gtk.Actions`), empty when it gave none.
        app_path: String,
        /// The menu bar's object path (`org.gtk.Menus`), empty when it gave none.
        menubar_path: String,
        /// The window's own object path (`org.gtk.Actions` for the window), empty when none.
        window_path: String,
    },
}

impl MenuSource {
    /// The kind as the wire names it.
    ///
    /// ```
    /// use casement_core::app::MenuSource;
    /// use casement_protocols::vocab::MenuSourceKind;
    ///
    /// assert_eq!(MenuSource::None.kind(), MenuSourceKind::None);
    /// ```
    pub fn kind(&self) -> MenuSourceKind {
        match self {
            MenuSource::None => MenuSourceKind::None,
            MenuSource::DbusMenu { .. } => MenuSourceKind::DbusMenu,
            MenuSource::GtkMenus { .. } => MenuSourceKind::GtkMenus,
        }
    }

    /// The bus name, empty for none.
    pub fn bus(&self) -> &str {
        match self {
            MenuSource::None => "",
            MenuSource::DbusMenu { bus, .. } | MenuSource::GtkMenus { bus, .. } => bus,
        }
    }

    /// The application object path, empty when the source has none (a dbusmenu has no app
    /// object).
    pub fn app_path(&self) -> &str {
        match self {
            MenuSource::GtkMenus { app_path, .. } => app_path,
            MenuSource::None | MenuSource::DbusMenu { .. } => "",
        }
    }

    /// The menu bar's object path: the dbusmenu object for Qt, the `org.gtk.Menus` path for GTK.
    pub fn menubar_path(&self) -> &str {
        match self {
            MenuSource::DbusMenu { path, .. } => path,
            MenuSource::GtkMenus { menubar_path, .. } => menubar_path,
            MenuSource::None => "",
        }
    }

    /// The path the shell is told for the window itself (`quire_desktop_window_v1.menu_path`):
    /// the dbusmenu object, or the GTK window's own object.
    pub fn window_path(&self) -> &str {
        match self {
            MenuSource::DbusMenu { path, .. } => path,
            MenuSource::GtkMenus { window_path, .. } => window_path,
            MenuSource::None => "",
        }
    }

    /// Whether the window declared a menu.
    pub fn is_declared(&self) -> bool {
        !matches!(self, MenuSource::None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_source_answers_the_wire_fields() {
        let qt = MenuSource::DbusMenu {
            bus: ":1.7".into(),
            path: "/MenuBar".into(),
        };
        let gtk = MenuSource::GtkMenus {
            bus: ":1.9".into(),
            app_path: "/org/gtk/Application".into(),
            menubar_path: "/org/gtk/Menus".into(),
            window_path: "/org/gtk/Window/1".into(),
        };
        let rows = [
            (
                "none",
                MenuSource::None,
                MenuSourceKind::None,
                ["", "", "", ""],
            ),
            (
                "qt",
                qt,
                MenuSourceKind::DbusMenu,
                [":1.7", "", "/MenuBar", "/MenuBar"],
            ),
            (
                "gtk",
                gtk,
                MenuSourceKind::GtkMenus,
                [
                    ":1.9",
                    "/org/gtk/Application",
                    "/org/gtk/Menus",
                    "/org/gtk/Window/1",
                ],
            ),
        ];
        for (name, source, kind, [bus, app, bar, window]) in rows {
            assert_eq!(source.kind(), kind, "{name}");
            assert_eq!(
                [
                    source.bus(),
                    source.app_path(),
                    source.menubar_path(),
                    source.window_path()
                ],
                [bus, app, bar, window],
                "{name}"
            );
        }
    }
}
