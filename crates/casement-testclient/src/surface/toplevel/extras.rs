//! The small `xdg` extensions around a toplevel (P0-H2): `xdg_wm_dialog_v1`,
//! `xdg_toplevel_icon_manager_v1`, `xdg_toplevel_tag_manager_v1`, `xdg_system_bell_v1` and
//! `xdg_foreign` v2 (exporter and importer). Each is a typed handle over the raw object; the
//! raw object stays public so a test can send the request the protocol forbids.

use super::{Host, Shared, Toplevel};
use crate::conn::TestClient;
use crate::shm::ShmBuffer;
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols::xdg::dialog::v1::client::xdg_dialog_v1::XdgDialogV1;
use wayland_protocols::xdg::dialog::v1::client::xdg_wm_dialog_v1::XdgWmDialogV1;
use wayland_protocols::xdg::foreign::zv2::client::zxdg_exported_v2::{self, ZxdgExportedV2};
use wayland_protocols::xdg::foreign::zv2::client::zxdg_exporter_v2::ZxdgExporterV2;
use wayland_protocols::xdg::foreign::zv2::client::zxdg_imported_v2::{self, ZxdgImportedV2};
use wayland_protocols::xdg::foreign::zv2::client::zxdg_importer_v2::ZxdgImporterV2;
use wayland_protocols::xdg::system_bell::v1::client::xdg_system_bell_v1::XdgSystemBellV1;
use wayland_protocols::xdg::toplevel_icon::v1::client::xdg_toplevel_icon_manager_v1::{
    self, XdgToplevelIconManagerV1,
};
use wayland_protocols::xdg::toplevel_icon::v1::client::xdg_toplevel_icon_v1::XdgToplevelIconV1;
use wayland_protocols::xdg::toplevel_tag::v1::client::xdg_toplevel_tag_manager_v1::XdgToplevelTagManagerV1;

pub use wayland_protocols::xdg::dialog::v1::client::xdg_wm_dialog_v1::Error as DialogError;
pub use wayland_protocols::xdg::toplevel_icon::v1::client::xdg_toplevel_icon_v1::Error as IconError;

/// A bound `xdg_wm_dialog_v1`.
#[derive(Debug)]
pub struct WmDialog {
    /// The global.
    pub manager: XdgWmDialogV1,
}

impl WmDialog {
    /// Binds the global (version 1).
    pub fn bind(client: &mut TestClient) -> WmDialog {
        WmDialog {
            manager: client
                .bind(1..=1, ())
                .expect("the compositor advertises xdg_wm_dialog_v1"),
        }
    }

    /// Makes `toplevel` a dialog (an `xdg_dialog_v1` object).
    pub fn dialog(&self, client: &mut TestClient, toplevel: &Toplevel) -> XdgDialogV1 {
        self.manager
            .get_xdg_dialog(&toplevel.toplevel, &client.qh(), ())
    }
}

/// What the icon manager advertised.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct IconSizes {
    /// Every `icon_size` sent.
    pub sizes: Vec<i32>,
    /// How many `done` events arrived.
    pub done: u32,
}

/// The icon manager's log.
pub type IconManagerLog = Shared<IconSizes>;

/// A bound `xdg_toplevel_icon_manager_v1`.
#[derive(Debug)]
pub struct IconManager {
    /// The global.
    pub manager: XdgToplevelIconManagerV1,
    /// What it advertised.
    pub log: IconManagerLog,
}

impl IconManager {
    /// Binds the global (version 1).
    pub fn bind(client: &mut TestClient) -> IconManager {
        let log = IconManagerLog::default();
        let manager = client
            .bind(1..=1, log.clone())
            .expect("the compositor advertises xdg_toplevel_icon_manager_v1");
        IconManager { manager, log }
    }

    /// An icon with `buffer` at `scale`, set on `toplevel`.
    pub fn set_icon(
        &self,
        client: &mut TestClient,
        toplevel: &Toplevel,
        buffer: &ShmBuffer,
        scale: i32,
    ) -> XdgToplevelIconV1 {
        let icon = self.manager.create_icon(&client.qh(), ());
        icon.add_buffer(buffer.buffer(), scale);
        self.manager.set_icon(&toplevel.toplevel, Some(&icon));
        icon
    }
}

/// A bound `xdg_toplevel_tag_manager_v1`.
#[derive(Debug)]
pub struct TagManager {
    /// The global.
    pub manager: XdgToplevelTagManagerV1,
}

impl TagManager {
    /// Binds the global (version 1).
    pub fn bind(client: &mut TestClient) -> TagManager {
        TagManager {
            manager: client
                .bind(1..=1, ())
                .expect("the compositor advertises xdg_toplevel_tag_manager_v1"),
        }
    }

    /// Tags `toplevel` and describes it.
    pub fn tag(&self, toplevel: &Toplevel, tag: &str, description: &str) {
        self.manager
            .set_toplevel_tag(&toplevel.toplevel, tag.to_owned());
        self.manager
            .set_toplevel_description(&toplevel.toplevel, description.to_owned());
    }
}

/// A bound `xdg_system_bell_v1`.
#[derive(Debug)]
pub struct SystemBell {
    /// The global.
    pub bell: XdgSystemBellV1,
}

impl SystemBell {
    /// Binds the global (version 1).
    pub fn bind(client: &mut TestClient) -> SystemBell {
        SystemBell {
            bell: client
                .bind(1..=1, ())
                .expect("the compositor advertises xdg_system_bell_v1"),
        }
    }

    /// Rings for `surface`, or for nobody in particular.
    pub fn ring(&self, surface: Option<&WlSurface>) {
        self.bell.ring(surface);
    }
}

/// The handle an exported toplevel was given, once the compositor sent it.
pub type ExportedHandle = Shared<Option<String>>;

/// What an imported toplevel was told.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ImportedEvents {
    /// How many `destroyed` events arrived (the export went away).
    pub destroyed: u32,
}

/// An imported toplevel's log.
pub type ImportedLog = Shared<ImportedEvents>;

/// Bound `zxdg_exporter_v2` and `zxdg_importer_v2`.
#[derive(Debug)]
pub struct Foreign {
    /// The exporter global.
    pub exporter: ZxdgExporterV2,
    /// The importer global.
    pub importer: ZxdgImporterV2,
}

impl Foreign {
    /// Binds both globals (version 1).
    pub fn bind(client: &mut TestClient) -> Foreign {
        Foreign {
            exporter: client
                .bind(1..=1, ())
                .expect("the compositor advertises zxdg_exporter_v2"),
            importer: client
                .bind(1..=1, ())
                .expect("the compositor advertises zxdg_importer_v2"),
        }
    }

    /// Exports `toplevel`; the handle arrives on the next pump.
    pub fn export(
        &self,
        client: &mut TestClient,
        toplevel: &Toplevel,
    ) -> (ZxdgExportedV2, ExportedHandle) {
        let handle = ExportedHandle::default();
        let exported =
            self.exporter
                .export_toplevel(toplevel.wl_surface(), &client.qh(), handle.clone());
        (exported, handle)
    }

    /// Imports `handle` and makes `child` its child.
    pub fn import_parent_of(
        &self,
        client: &mut TestClient,
        handle: &str,
        child: &Toplevel,
    ) -> (ZxdgImportedV2, ImportedLog) {
        let log = ImportedLog::default();
        let imported = self
            .importer
            .import_toplevel(handle.to_owned(), &client.qh(), log.clone());
        imported.set_parent_of(child.wl_surface());
        (imported, log)
    }
}

/// Implements `Dispatch` with no events for objects that have none (or whose events the tests
/// do not read).
macro_rules! silent {
    ($($object:ty, $data:ty);* $(;)?) => {$(
        impl Dispatch<$object, $data> for Host {
            fn event(
                _: &mut Self,
                _: &$object,
                _: <$object as Proxy>::Event,
                _: &$data,
                _: &Connection,
                _: &QueueHandle<Self>,
            ) {
            }
        }
    )*};
}

silent! {
    XdgWmDialogV1, ();
    XdgDialogV1, ();
    XdgToplevelIconV1, ();
    XdgToplevelTagManagerV1, ();
    XdgSystemBellV1, ();
    ZxdgExporterV2, ();
    ZxdgImporterV2, ();
}

impl Dispatch<XdgToplevelIconManagerV1, IconManagerLog> for Host {
    fn event(
        _: &mut Self,
        _: &XdgToplevelIconManagerV1,
        event: xdg_toplevel_icon_manager_v1::Event,
        log: &IconManagerLog,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            xdg_toplevel_icon_manager_v1::Event::IconSize { size } => {
                log.with(|seen| seen.sizes.push(size));
            }
            xdg_toplevel_icon_manager_v1::Event::Done => {
                log.with(|seen| seen.done += 1);
            }
            _ => {}
        }
    }
}

impl Dispatch<ZxdgExportedV2, ExportedHandle> for Host {
    fn event(
        _: &mut Self,
        _: &ZxdgExportedV2,
        event: zxdg_exported_v2::Event,
        handle: &ExportedHandle,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let zxdg_exported_v2::Event::Handle { handle: text } = event {
            // The handle is random, so the transcript names it without its text.
            handle.with(|seen| *seen = Some(text));
        }
    }
}

impl Dispatch<ZxdgImportedV2, ImportedLog> for Host {
    fn event(
        _: &mut Self,
        _: &ZxdgImportedV2,
        event: zxdg_imported_v2::Event,
        log: &ImportedLog,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if matches!(event, zxdg_imported_v2::Event::Destroyed) {
            log.with(|seen| seen.destroyed += 1);
        }
    }
}
