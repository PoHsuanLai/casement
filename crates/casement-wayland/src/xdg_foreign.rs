//! `zxdg_exporter_v2` and `zxdg_importer_v2`: parenting across clients (a portal dialog over
//! an app). An imported parent reaches the model through `xdg_shell::on_parent`, so this module
//! has no request functions (the plan listed `on_export`/`on_import`: FINDINGS).
//!
//! The protocol is served here, not by smithay's module: smithay never sends the importers
//! `zxdg_imported_v2.destroyed` when an export goes away (the protocol says it must, and an
//! importer waits for it to drop its dialog's parent), and it draws its handles from a thread
//! random number generator. What each request means is in [`book`] (the exports and who
//! imported them); this file is the wire: the part, the dispatch impls and their handler bound.

mod book;

pub use book::Handle;

use crate::limits::{self, Limit};
use crate::states::GlobalFilter;
use book::Shared;
use smithay::reexports::wayland_protocols::xdg::foreign::zv2::server::zxdg_exported_v2::ZxdgExportedV2;
use smithay::reexports::wayland_protocols::xdg::foreign::zv2::server::zxdg_exporter_v2::{
    self, ZxdgExporterV2,
};
use smithay::reexports::wayland_protocols::xdg::foreign::zv2::server::zxdg_imported_v2::{
    self, ZxdgImportedV2,
};
use smithay::reexports::wayland_protocols::xdg::foreign::zv2::server::zxdg_importer_v2::{
    self, ZxdgImporterV2,
};
use smithay::reexports::wayland_server::backend::{ClientId, GlobalId};
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::{
    Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch, New, Resource,
};
use smithay::wayland::compositor::{get_role, with_states};
use smithay::wayland::shell::is_valid_parent;
use smithay::wayland::shell::xdg::{XDG_TOPLEVEL_ROLE, XdgShellHandler, XdgToplevelSurfaceData};
use smithay::wayland::{Dispatch2, GlobalDispatch2};

/// The version casement serves, of both globals.
pub const VERSION: u32 = 1;

/// `zxdg_exporter_v2` and `zxdg_importer_v2`.
#[derive(Debug)]
pub struct XdgForeignPart {
    /// The exporter global.
    pub exporter: GlobalId,
    /// The importer global.
    pub importer: GlobalId,
    /// Every export and what was imported from it.
    book: Shared,
}

impl XdgForeignPart {
    /// Creates both globals (version 1). Every connection sees them.
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> XdgForeignPart
    where
        D: GlobalDispatch<ZxdgExporterV2, ForeignGlobal>
            + GlobalDispatch<ZxdgImporterV2, ForeignGlobal>
            + 'static,
    {
        let _ = role_filter;
        let book = Shared::default();
        let global = ForeignGlobal { book: book.clone() };
        XdgForeignPart {
            exporter: display.create_global::<D, ZxdgExporterV2, _>(VERSION, global.clone()),
            importer: display.create_global::<D, ZxdgImporterV2, _>(VERSION, global),
            book,
        }
    }
}

/// A toplevel's surface is gone: what was exported from it is no longer valid, and the
/// importers are told so.
pub fn on_surface_destroyed(part: &XdgForeignPart, surface: &WlSurface) {
    for imported in part.book.lock().surface_destroyed(surface) {
        imported.destroyed();
    }
}

/// The user data of both globals.
#[derive(Debug, Clone)]
pub struct ForeignGlobal {
    book: Shared,
}

/// The user data of a bound exporter.
#[derive(Debug, Clone)]
pub struct Exporter {
    book: Shared,
}

/// The user data of a bound importer.
#[derive(Debug, Clone)]
pub struct Importer {
    book: Shared,
}

/// The user data of an `xdg_exported`: its handle.
#[derive(Debug, Clone)]
pub struct Exported {
    handle: Handle,
    book: Shared,
}

/// The user data of an `xdg_imported`: the handle it asked for.
#[derive(Debug, Clone)]
pub struct Imported {
    handle: Handle,
    book: Shared,
}

impl<D> GlobalDispatch2<ZxdgExporterV2, D> for ForeignGlobal
where
    D: Dispatch<ZxdgExporterV2, Exporter> + 'static,
{
    fn bind(
        &self,
        _state: &mut D,
        _display: &DisplayHandle,
        _client: &Client,
        resource: New<ZxdgExporterV2>,
        data_init: &mut DataInit<'_, D>,
    ) {
        data_init.init(
            resource,
            Exporter {
                book: self.book.clone(),
            },
        );
    }
}

impl<D> GlobalDispatch2<ZxdgImporterV2, D> for ForeignGlobal
where
    D: Dispatch<ZxdgImporterV2, Importer> + 'static,
{
    fn bind(
        &self,
        _state: &mut D,
        _display: &DisplayHandle,
        _client: &Client,
        resource: New<ZxdgImporterV2>,
        data_init: &mut DataInit<'_, D>,
    ) {
        data_init.init(
            resource,
            Importer {
                book: self.book.clone(),
            },
        );
    }
}

impl<D> Dispatch2<ZxdgExporterV2, D> for Exporter
where
    D: Dispatch<ZxdgExportedV2, Exported> + 'static,
{
    fn request(
        &self,
        _state: &mut D,
        client: &Client,
        exporter: &ZxdgExporterV2,
        request: zxdg_exporter_v2::Request,
        display: &DisplayHandle,
        data_init: &mut DataInit<'_, D>,
    ) {
        let zxdg_exporter_v2::Request::ExportToplevel { id, surface } = request else {
            return;
        };
        if get_role(&surface) != Some(XDG_TOPLEVEL_ROLE) {
            // The new object still gets data (a request on an object without any panics the
            // backend); the client is gone with the error, and nothing is exported.
            data_init.init(
                id,
                Exported {
                    handle: Handle::none(),
                    book: self.book.clone(),
                },
            );
            exporter.post_error(
                zxdg_exporter_v2::Error::InvalidSurface,
                "exported surface had an invalid role",
            );
            return;
        }
        // A client past its cap is cut (CASW-06); the new object still gets data, as above.
        let held = self.book.lock().held_by(&client.id());
        if !limits::admit(display, client, Limit::Exports, held) {
            data_init.init(
                id,
                Exported {
                    handle: Handle::none(),
                    book: self.book.clone(),
                },
            );
            return;
        }
        let handle = self.book.lock().export(surface);
        let exported = data_init.init(
            id,
            Exported {
                handle: handle.clone(),
                book: self.book.clone(),
            },
        );
        exported.handle(handle.as_str().to_owned());
    }
}

impl<D> Dispatch2<ZxdgExportedV2, D> for Exported
where
    D: XdgShellHandler + 'static,
{
    fn request(
        &self,
        _state: &mut D,
        _client: &Client,
        _exported: &ZxdgExportedV2,
        _request: <ZxdgExportedV2 as Resource>::Request,
        _display: &DisplayHandle,
        _data_init: &mut DataInit<'_, D>,
    ) {
    }

    /// The export is revoked: every importer is told, and the dialogs they parented lose it.
    fn destroyed(&self, state: &mut D, _client: ClientId, _exported: &ZxdgExportedV2) {
        let Some(revoked) = self.book.lock().revoke(&self.handle) else {
            return;
        };
        for imported in &revoked.imports {
            imported.destroyed();
        }
        orphan(state, revoked.orphans);
    }
}

impl<D> Dispatch2<ZxdgImporterV2, D> for Importer
where
    D: Dispatch<ZxdgImportedV2, Imported> + 'static,
{
    fn request(
        &self,
        _state: &mut D,
        _client: &Client,
        _importer: &ZxdgImporterV2,
        request: zxdg_importer_v2::Request,
        _display: &DisplayHandle,
        data_init: &mut DataInit<'_, D>,
    ) {
        let zxdg_importer_v2::Request::ImportToplevel { id, handle } = request else {
            return;
        };
        let handle = Handle::of(handle);
        let imported = data_init.init(
            id,
            Imported {
                handle: handle.clone(),
                book: self.book.clone(),
            },
        );
        if !self.book.lock().import(&handle, imported.clone()) {
            imported.destroyed();
        }
    }
}

impl<D> Dispatch2<ZxdgImportedV2, D> for Imported
where
    D: XdgShellHandler + 'static,
{
    fn request(
        &self,
        state: &mut D,
        _client: &Client,
        imported: &ZxdgImportedV2,
        request: zxdg_imported_v2::Request,
        _display: &DisplayHandle,
        _data_init: &mut DataInit<'_, D>,
    ) {
        let zxdg_imported_v2::Request::SetParentOf { surface: child } = request else {
            return;
        };
        let Some(parent) = self.book.lock().exported_surface(&self.handle) else {
            return;
        };
        let is_toplevel = with_states(&child, |states| {
            states.data_map.get::<XdgToplevelSurfaceData>().is_some()
        });
        if !is_toplevel || !is_valid_parent(&child, &parent) {
            imported.post_error(
                zxdg_imported_v2::Error::InvalidSurface,
                "invalid parent relationship",
            );
            return;
        }
        let changed = with_states(&child, |states| {
            let Some(data) = states.data_map.get::<XdgToplevelSurfaceData>() else {
                return false;
            };
            let Ok(mut role) = data.lock() else {
                return false;
            };
            let changed = role.parent.as_ref() != Some(&parent);
            role.parent = Some(parent.clone());
            changed
        });
        self.book
            .lock()
            .adopt(&self.handle, child.clone(), imported.clone());
        if changed {
            orphan_changed(state, &child);
        }
    }

    /// The import is gone: the dialogs it parented lose the parent.
    fn destroyed(&self, state: &mut D, _client: ClientId, imported: &ZxdgImportedV2) {
        let orphans = self.book.lock().release(&self.handle, imported);
        orphan(state, orphans);
    }
}

/// `children` lose the parent they were given from an export: the surfaces still holding it
/// drop it, and the model hears of it as any parent change.
fn orphan<D: XdgShellHandler>(state: &mut D, orphans: Vec<(WlSurface, WlSurface)>) {
    for (child, parent) in orphans {
        let cleared = with_states(&child, |states| {
            let Some(data) = states.data_map.get::<XdgToplevelSurfaceData>() else {
                return false;
            };
            let Ok(mut role) = data.lock() else {
                return false;
            };
            let held = role.parent.as_ref() == Some(&parent);
            if held {
                role.parent = None;
            }
            held
        });
        if cleared {
            orphan_changed(state, &child);
        }
    }
}

/// Tells the shell handler that `child`'s parent changed.
fn orphan_changed<D: XdgShellHandler>(state: &mut D, child: &WlSurface) {
    let toplevel = state
        .xdg_shell_state()
        .toplevel_surfaces()
        .iter()
        .find(|toplevel| toplevel.wl_surface() == child)
        .cloned();
    if let Some(toplevel) = toplevel {
        state.parent_changed(toplevel);
    }
}
