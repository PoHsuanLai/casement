//! The surface book follows the model's events: a toplevel's is [`super::windows`]'s; a
//! layer surface that maps or changes is drawn at its committed size, one that unmaps is not;
//! the lock surfaces are drawn over their outputs.

use crate::state::State;
use casement_core::geometry::LogicalSize;
use casement_core::{Event, OutputKey};
use casement_render::SurfaceEntry;
use casement_wayland::fractional_scale;
use ds_core::geometry::scale::Scale;
use smithay::desktop::utils::bbox_from_surface_tree;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::wayland::compositor::{
    RegionAttributes, SurfaceAttributes, TraversalAction, with_states, with_surface_tree_downward,
};

/// Brings the surface book up to date with `events`, in order.
pub(super) fn sync(state: &mut State, events: &[Event]) {
    for event in events {
        match event {
            Event::ToplevelMapped(_)
            | Event::ToplevelCommitted { .. }
            | Event::ToplevelUnmapped { .. } => super::windows::sync(state, event),
            Event::LayerMapped { layer, .. } | Event::LayerChanged { layer, .. } => {
                let Some(root) = state.wl.layer_shell.surface_of(*layer).cloned() else {
                    continue;
                };
                let size = bbox_from_surface_tree(&root, (0, 0)).size;
                if let Event::LayerMapped { spec, .. } = event {
                    // At info: a layer that covers a point takes the pointer there unless its
                    // input region leaves the point out, so which layer it is and what region it
                    // set is what tells a refused click's `target=Layer(_)` apart.
                    tracing::info!(
                        ?layer,
                        namespace = %spec.namespace,
                        level = ?spec.level,
                        ?size,
                        input = ?input_region(&root),
                        "layer mapped"
                    );
                }
                let entry = SurfaceEntry {
                    root,
                    committed: LogicalSize {
                        w: u32::try_from(size.w).unwrap_or(0),
                        h: u32::try_from(size.h).unwrap_or(0),
                    },
                };
                let book = std::mem::take(&mut state.render.textures);
                state.render.textures = book.with_layer(*layer, entry);
            }
            Event::LayerUnmapped { layer } => {
                // Whatever shell role the surface held is free again.
                casement_wayland::quire::shell::on_layer_gone(&mut state.wl.quire_shell, *layer);
                let book = std::mem::take(&mut state.render.textures);
                state.render.textures = book.without_layer(layer);
            }
            Event::Lock(_) => sync_locks(state),
            _ => {}
        }
    }
}

/// The input region `root` committed; `None` is the whole surface.
fn input_region(root: &WlSurface) -> Option<RegionAttributes> {
    with_states(root, |states| {
        states
            .cached_state
            .get::<SurfaceAttributes>()
            .current()
            .input_region
            .clone()
    })
}

/// Each output's lock surface is drawn at its output's size.
fn sync_locks(state: &mut State) {
    let mut book = std::mem::take(&mut state.render.textures).without_locks();
    for (output, lock) in &state.wl.session_lock.surfaces {
        let Some(plugged) = state.core.model.outputs().0.get(output) else {
            continue;
        };
        let entry = SurfaceEntry {
            root: lock.wl_surface().clone(),
            committed: plugged.config.logical_rect().size,
        };
        book = book.with_lock(output.clone(), entry);
    }
    state.render.textures = book;
}

/// `output` now has `scale`: every surface on it (its windows, its layer surfaces, its lock
/// surface, each with its subsurfaces) is told its preferred scale again (smithay sends only
/// what changed).
pub(crate) fn resend_scale(state: &mut State, output: &OutputKey, scale: Scale) {
    let roots: Vec<WlSurface> = surfaces_on(state, output);
    for root in roots {
        let mut tree = Vec::new();
        with_surface_tree_downward(
            &root,
            (),
            |_, _, ()| TraversalAction::DoChildren(()),
            |surface, _, ()| tree.push(surface.clone()),
            |_, _, ()| true,
        );
        for surface in tree {
            if fractional_scale::apply_scale(&mut state.wl.fractional_scale, &surface, scale)
                .is_err()
            {
                tracing::debug!("a surface was destroyed before it heard its new scale");
            }
        }
    }
}

/// The root surfaces on `output`.
fn surfaces_on(state: &State, output: &OutputKey) -> Vec<WlSurface> {
    let windows = state
        .core
        .arrangement
        .windows
        .iter()
        .filter(|target| target.output == *output)
        .filter_map(|target| state.wl.xdg_shell.toplevels.get(&target.window))
        .map(|toplevel| toplevel.wl_surface().clone());
    let layers = state
        .core
        .model
        .layers()
        .iter()
        .filter(|(_, spec)| spec.output == *output)
        .filter_map(|(key, _)| state.wl.layer_shell.surface_of(*key).cloned());
    let lock = state
        .wl
        .session_lock
        .surfaces
        .get(output)
        .map(|lock| lock.wl_surface().clone());
    windows.chain(layers).chain(lock).collect()
}
