//! The exports: which toplevel each handle names, who imported it, and which dialogs were
//! parented through it. Plain data behind a lock the objects share; the rules (what a revoke
//! hands back, who becomes an orphan) are methods the protocol tests drive through real
//! surfaces (a `WlSurface` cannot be made without a client).

use smithay::reexports::wayland_protocols::xdg::foreign::zv2::server::zxdg_imported_v2::ZxdgImportedV2;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use std::collections::BTreeMap;
use std::hash::BuildHasher;
use std::sync::{Arc, Mutex, MutexGuard};

/// What an exported toplevel is called on the wire: 32 hex digits, drawn from std's random
/// hasher keys (a handle is a capability to parent a window to another app's, so it is not
/// guessable), never from the order of anything.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Handle(String);

impl Handle {
    /// The handle of nothing (the object a refused export leaves behind).
    pub(super) fn none() -> Handle {
        Handle(String::new())
    }

    /// The handle a client named.
    pub(super) fn of(text: String) -> Handle {
        Handle(text)
    }

    /// The wire text.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn minted(counter: u64) -> Handle {
        let draw =
            |salt: u64| std::collections::hash_map::RandomState::new().hash_one((counter, salt));
        Handle(format!("{:016x}{:016x}", draw(1), draw(2)))
    }
}

/// One exported toplevel.
#[derive(Debug)]
struct Export {
    /// The exported toplevel's surface.
    surface: WlSurface,
    /// Everyone who imported the handle.
    imports: Vec<ZxdgImportedV2>,
    /// The dialogs parented through an import, and the import that did it.
    children: Vec<(WlSurface, ZxdgImportedV2)>,
}

/// What revoking an export hands back.
#[derive(Debug)]
pub(super) struct Revoked {
    /// The imports to tell they are gone.
    pub(super) imports: Vec<ZxdgImportedV2>,
    /// (child, parent) of every dialog that was parented through the export.
    pub(super) orphans: Vec<(WlSurface, WlSurface)>,
}

/// Every export, by handle.
#[derive(Debug, Default)]
pub(super) struct Book {
    exports: BTreeMap<Handle, Export>,
    minted: u64,
}

/// The book the globals and objects of one display share.
#[derive(Debug, Clone, Default)]
pub(super) struct Shared(Arc<Mutex<Book>>);

impl Shared {
    pub(super) fn lock(&self) -> MutexGuard<'_, Book> {
        // The book is plain data: a panic elsewhere left it whole.
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl Book {
    /// `surface` is exported: its new handle.
    pub(super) fn export(&mut self, surface: WlSurface) -> Handle {
        let handle = loop {
            self.minted += 1;
            let handle = Handle::minted(self.minted);
            if !self.exports.contains_key(&handle) {
                break handle;
            }
        };
        self.exports.insert(
            handle.clone(),
            Export {
                surface,
                imports: Vec::new(),
                children: Vec::new(),
            },
        );
        handle
    }

    /// The toplevel `handle` names.
    pub(super) fn exported_surface(&self, handle: &Handle) -> Option<WlSurface> {
        self.exports
            .get(handle)
            .map(|export| export.surface.clone())
    }

    /// `imported` imports `handle`; false when nothing is exported under it.
    pub(super) fn import(&mut self, handle: &Handle, imported: ZxdgImportedV2) -> bool {
        match self.exports.get_mut(handle) {
            Some(export) => {
                export.imports.push(imported);
                true
            }
            None => false,
        }
    }

    /// `child` is parented through `imported`.
    pub(super) fn adopt(&mut self, handle: &Handle, child: WlSurface, imported: ZxdgImportedV2) {
        if let Some(export) = self.exports.get_mut(handle) {
            export.children.retain(|(held, _)| *held != child);
            export.children.push((child, imported));
        }
    }

    /// The export is revoked (its `xdg_exported` was destroyed).
    pub(super) fn revoke(&mut self, handle: &Handle) -> Option<Revoked> {
        let export = self.exports.remove(handle)?;
        let parent = export.surface;
        Some(Revoked {
            imports: export.imports,
            orphans: export
                .children
                .into_iter()
                .map(|(child, _)| (child, parent.clone()))
                .collect(),
        })
    }

    /// `imported` is gone: it no longer imports, and the dialogs it parented are orphans of the
    /// exported toplevel.
    pub(super) fn release(
        &mut self,
        handle: &Handle,
        imported: &ZxdgImportedV2,
    ) -> Vec<(WlSurface, WlSurface)> {
        let Some(export) = self.exports.get_mut(handle) else {
            return Vec::new();
        };
        export.imports.retain(|held| held != imported);
        let (gone, kept): (Vec<_>, Vec<_>) = std::mem::take(&mut export.children)
            .into_iter()
            .partition(|(_, by)| by == imported);
        export.children = kept;
        gone.into_iter()
            .map(|(child, _)| (child, export.surface.clone()))
            .collect()
    }

    /// The toplevel `surface` is destroyed: what it exported is revoked, and the imports of
    /// those exports are returned to be told. A dialog it was the parent of needs nothing: the
    /// shell drops a destroyed parent itself.
    pub(super) fn surface_destroyed(&mut self, surface: &WlSurface) -> Vec<ZxdgImportedV2> {
        self.exports
            .values_mut()
            .filter(|export| export.surface == *surface)
            .flat_map(|export| {
                export.children.clear();
                std::mem::take(&mut export.imports)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A handle is 32 hex digits and two exports never share one.
    #[test]
    fn handles_are_long_and_distinct() {
        let (a, b) = (Handle::minted(1), Handle::minted(2));
        assert_eq!(a.as_str().len(), 32);
        assert!(a.as_str().bytes().all(|byte| byte.is_ascii_hexdigit()));
        assert_ne!(a, b);
    }

    /// Nothing is exported under a handle nobody minted, and revoking it hands back nothing.
    #[test]
    fn an_unknown_handle_names_nothing() {
        let mut book = Book::default();
        let handle = Handle::of("nobody".to_owned());
        assert!(book.exported_surface(&handle).is_none());
        assert!(book.revoke(&handle).is_none());
    }
}
