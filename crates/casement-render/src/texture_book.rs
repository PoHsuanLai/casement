//! Which surface tree draws each window, layer surface and lock surface.

use casement_core::geometry::LogicalSize;
use casement_core::{LayerKey, OutputKey, WindowId};
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use std::collections::BTreeMap;

/// One root surface and the size its last commit had.
#[derive(Debug, Clone, PartialEq)]
pub struct SurfaceEntry {
    /// The root surface (its subsurfaces and popups hang off it).
    pub root: WlSurface,
    /// The size of its last committed buffer, in logical pixels.
    pub committed: LogicalSize,
}

/// Every surface tree the renderer may draw, keyed by what the model calls it. Filled by the
/// protocol handlers as surfaces map and unmap; read by [`crate::output_elements`].
#[derive(Debug, Clone, Default)]
pub struct TextureBook {
    windows: BTreeMap<WindowId, SurfaceEntry>,
    layers: BTreeMap<LayerKey, SurfaceEntry>,
    locks: BTreeMap<OutputKey, SurfaceEntry>,
    cursor: Option<WlSurface>,
}

impl TextureBook {
    /// No surfaces.
    pub fn empty() -> TextureBook {
        TextureBook::default()
    }

    /// The book with `window` drawn from `entry`.
    pub fn with_window(mut self, window: WindowId, entry: SurfaceEntry) -> TextureBook {
        self.windows.insert(window, entry);
        self
    }

    /// The book without `window`.
    pub fn without_window(mut self, window: &WindowId) -> TextureBook {
        self.windows.remove(window);
        self
    }

    /// The book with `layer` drawn from `entry`.
    pub fn with_layer(mut self, layer: LayerKey, entry: SurfaceEntry) -> TextureBook {
        self.layers.insert(layer, entry);
        self
    }

    /// The book without `layer`.
    pub fn without_layer(mut self, layer: &LayerKey) -> TextureBook {
        self.layers.remove(layer);
        self
    }

    /// The book with every layer surface's committed size read again by `size_of` (a layer
    /// surface's buffer arrives on a commit that changes nothing the model hears about).
    pub fn with_layers_sized(mut self, size_of: impl Fn(&WlSurface) -> LogicalSize) -> TextureBook {
        self.layers
            .values_mut()
            .for_each(|entry| entry.committed = size_of(&entry.root));
        self
    }

    /// The book with `output`'s lock surface drawn from `entry`.
    pub fn with_lock(mut self, output: OutputKey, entry: SurfaceEntry) -> TextureBook {
        self.locks.insert(output, entry);
        self
    }

    /// The book without any lock surface.
    pub fn without_locks(mut self) -> TextureBook {
        self.locks.clear();
        self
    }

    /// The book with the client cursor surface `cursor` (or none).
    pub fn with_cursor(mut self, cursor: Option<WlSurface>) -> TextureBook {
        self.cursor = cursor;
        self
    }

    /// `window`'s surface tree.
    pub fn window(&self, window: &WindowId) -> Option<&SurfaceEntry> {
        self.windows.get(window)
    }

    /// `layer`'s surface tree.
    pub fn layer(&self, layer: &LayerKey) -> Option<&SurfaceEntry> {
        self.layers.get(layer)
    }

    /// `output`'s lock surface.
    pub fn lock(&self, output: &OutputKey) -> Option<&SurfaceEntry> {
        self.locks.get(output)
    }

    /// The client cursor surface.
    pub fn cursor(&self) -> Option<&WlSurface> {
        self.cursor.as_ref()
    }
}
