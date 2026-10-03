//! `wl_subcompositor`: subsurfaces. The global comes with `wl_compositor`
//! ([`crate::compositor::CompositorPart`]); this module holds what subsurfaces add.
//! smithay keeps the tree, the stacking order (`place_above`, `place_below`), synchronised
//! commits and the `bad_surface`/`bad_parent` protocol errors.

use casement_core::Event;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;

/// What casement keeps about subsurfaces (nothing beyond smithay's tree at P0).
#[derive(Debug, Default)]
pub struct SubcompositorPart;

impl SubcompositorPart {
    /// The part: its global is created with `wl_compositor`'s.
    pub fn new() -> SubcompositorPart {
        SubcompositorPart
    }
}

/// A subsurface of `parent` was created. The model knows windows, not surface trees, so
/// nothing reaches it; the renderer reads the tree from smithay when it draws the root.
pub fn on_new_subsurface(
    part: &mut SubcompositorPart,
    surface: &WlSurface,
    parent: &WlSurface,
) -> Vec<Event> {
    let _ = (part, surface, parent);
    Vec::new()
}
