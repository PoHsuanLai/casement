//! The surface book's windows: a toplevel that maps is drawn from its surface tree at its
//! committed size, one that commits again keeps the new size, one that unmaps is not drawn.

use crate::state::State;
use casement_core::geometry::LogicalSize;
use casement_core::{Event, WindowId};
use casement_render::{SurfaceEntry, geometry_size};
use smithay::desktop::utils::bbox_from_surface_tree;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;

/// The book follows `event` when it is about a toplevel.
pub(super) fn sync(state: &mut State, event: &Event) {
    match event {
        Event::ToplevelMapped(mapped) => draw(state, mapped.window, None),
        Event::ToplevelCommitted { window, size } => draw(state, *window, Some(*size)),
        Event::ToplevelUnmapped { window } => {
            let book = std::mem::take(&mut state.render.textures);
            state.render.textures = book.without_window(window);
        }
        _ => {}
    }
}

/// Every layer surface in the book is drawn at the size its surface tree has now: its buffer
/// may have arrived on a commit that changed nothing the model was told.
pub(super) fn refresh_layers(state: &mut State) {
    let book = std::mem::take(&mut state.render.textures);
    state.render.textures = book.with_layers_sized(tree_size);
}

/// The size of a surface tree, in logical pixels: its window geometry when it declared one (what
/// a window's size is), else everything it draws.
fn tree_size(root: &WlSurface) -> LogicalSize {
    let tree = geometry_size(root).unwrap_or_else(|| bbox_from_surface_tree(root, (0, 0)).size);
    LogicalSize {
        w: u32::try_from(tree.w).unwrap_or(0),
        h: u32::try_from(tree.h).unwrap_or(0),
    }
}

/// `window` is drawn from its root surface, at `size` when the event said so, else at the size
/// of its surface tree. A window the protocol part no longer holds is not drawn.
fn draw(state: &mut State, window: WindowId, size: Option<LogicalSize>) {
    let Some(root) = state
        .wl
        .xdg_shell
        .toplevels
        .get(&window)
        .map(|toplevel| toplevel.wl_surface().clone())
    else {
        return;
    };
    let committed = size.unwrap_or_else(|| tree_size(&root));
    let book = std::mem::take(&mut state.render.textures);
    state.render.textures = book.with_window(window, SurfaceEntry { root, committed });
}
