//! Which outputs each drawn surface is on: `wl_surface.enter` and `leave`, and the preferred
//! buffer scale of the output a surface is mostly on.
//!
//! A surface enters an output when its drawn rectangle intersects the output's logical
//! rectangle, and leaves when it no longer does, when it unmaps, or when the output goes. The
//! surfaces are the hit-test stack's (lock surfaces, layer surfaces, windows, each with its
//! popups), placed where the stack places them, so what a client is told is what the person
//! sees. A root's subsurfaces follow it through smithay's `output_update`; a popup is on the
//! outputs its parent is on and on those its own rectangle touches.

use super::scene::{Hit, stack};
use crate::state::State;
use casement_core::OutputKey;
use casement_core::geometry::{LogicalPoint, LogicalRect};
use casement_wayland::fractional_scale;
use casement_wayland::seat::Candidate;
use smithay::backend::renderer::utils::RendererSurfaceStateUserData;
use smithay::desktop::utils::bbox_from_surface_tree;
use smithay::output::Output;
use smithay::reexports::wayland_server::Resource;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::utils::{Logical, Point, Rectangle, Size};
use smithay::wayland::compositor::{TraversalAction, with_surface_tree_downward};
use std::collections::BTreeMap;

/// The surfaces that were told they are on an output at the last update, so one that has
/// since left the stack can be told it left.
#[derive(Debug, Default)]
pub struct Presence {
    seen: Vec<WlSurface>,
}

/// One surface tree of the stack and where it stands.
struct Placed {
    root: WlSurface,
    /// Where it is drawn, in global logical pixels.
    drawn: Rectangle<i32, Logical>,
    origin: LogicalPoint,
    /// What it hangs from: the first of its group is a popup, the last the root.
    popup: bool,
}

/// Brings every surface's outputs up to date with where the stack puts it.
pub(super) fn update(state: &mut State) {
    let outputs = outputs_of(state);
    let placed = placements(state);
    let mut roots_seen = Vec::new();
    for group in placed.chunk_by(|a, b| a.0 == b.0) {
        let Some(root) = group.iter().find(|(_, placed)| !placed.popup) else {
            continue;
        };
        let home = home_of(&root.1, &outputs);
        let on_root: Vec<&OutputKey> = outputs
            .iter()
            .filter(|(_, (_, rect))| touches(&root.1.drawn, rect))
            .map(|(key, _)| key)
            .collect();
        for (_, surface) in group {
            tell_outputs(surface, &on_root, &outputs);
            tell_scale(state, surface, home.as_ref(), &outputs);
            roots_seen.push(surface.root.clone());
        }
    }
    let gone: Vec<WlSurface> = std::mem::take(&mut state.render.presence.seen)
        .into_iter()
        .filter(|surface| !roots_seen.contains(surface))
        .collect();
    for surface in gone.iter().filter(|surface| surface.is_alive()) {
        outputs
            .values()
            .for_each(|(output, _)| output_update(output, None, surface));
    }
    state.render.presence.seen = roots_seen;
}

/// The advertised outputs with the logical rectangle the model puts each at.
fn outputs_of(state: &State) -> BTreeMap<OutputKey, (Output, Rectangle<i32, Logical>)> {
    state
        .wl
        .output
        .outputs
        .iter()
        .filter_map(|(key, output)| {
            let rect = state.core.model.outputs().0.get(key)?.config.logical_rect();
            Some((key.clone(), (output.clone(), rectangle(rect))))
        })
        .collect()
}

/// smithay's rectangle for ours.
fn rectangle(rect: LogicalRect) -> Rectangle<i32, Logical> {
    Rectangle::new(
        Point::from((rect.origin.x, rect.origin.y)),
        Size::from((
            i32::try_from(rect.size.w).unwrap_or(i32::MAX),
            i32::try_from(rect.size.h).unwrap_or(i32::MAX),
        )),
    )
}

/// Whether two rectangles share area (an empty one touches nothing).
fn touches(a: &Rectangle<i32, Logical>, b: &Rectangle<i32, Logical>) -> bool {
    !a.is_empty() && !b.is_empty() && a.overlaps(*b)
}

/// Every drawn tree of the stack, each with its group's `Hit`, popups before their root.
fn placements(state: &State) -> Vec<(Hit, Placed)> {
    stack(state)
        .into_iter()
        .map(|(hit, candidate)| {
            let drawn = drawn_at(&candidate);
            let popup = is_popup(state, &hit, &candidate);
            (
                hit,
                Placed {
                    drawn,
                    origin: candidate.origin,
                    root: candidate.root,
                    popup,
                },
            )
        })
        .collect()
}

/// A candidate is a popup when it is not the surface its hit's owner draws: the roots are the
/// ones a popup list hangs from.
fn is_popup(state: &State, hit: &Hit, candidate: &Candidate) -> bool {
    let book = &state.render.textures;
    let root = match hit {
        Hit::Window(id) => book.window(id).map(|entry| &entry.root),
        Hit::Layer(key) => book.layer(key).map(|entry| &entry.root),
        Hit::Lock(output) => book.lock(output).map(|entry| &entry.root),
    };
    root != Some(&candidate.root)
}

/// Where a candidate's tree is drawn, in global logical pixels.
fn drawn_at(candidate: &Candidate) -> Rectangle<i32, Logical> {
    let mut bbox = bbox_from_surface_tree(&candidate.root, (0, 0));
    bbox.loc += Point::from((candidate.origin.x, candidate.origin.y));
    bbox
}

/// The output a root is mostly on (the largest overlap; the first by key on a tie).
fn home_of(
    root: &Placed,
    outputs: &BTreeMap<OutputKey, (Output, Rectangle<i32, Logical>)>,
) -> Option<OutputKey> {
    outputs
        .iter()
        .filter_map(|(key, (_, rect))| {
            let overlap = root.drawn.intersection(*rect)?;
            Some((i64::from(overlap.size.w) * i64::from(overlap.size.h), key))
        })
        .filter(|(area, _)| *area > 0)
        .fold(None, |best: Option<(i64, &OutputKey)>, found| match best {
            Some(held) if held.0 >= found.0 => Some(held),
            _ => Some(found),
        })
        .map(|(_, key)| key.clone())
}

/// Enters and leaves `surface`'s tree on each output.
fn tell_outputs(
    surface: &Placed,
    on_root: &[&OutputKey],
    outputs: &BTreeMap<OutputKey, (Output, Rectangle<i32, Logical>)>,
) {
    for (key, (output, rect)) in outputs {
        if surface.popup {
            let inherits = !surface.drawn.is_empty()
                && (on_root.contains(&key) || touches(&surface.drawn, rect));
            for_each_surface(&surface.root, |one| {
                if inherits {
                    output.enter(one);
                } else {
                    output.leave(one);
                }
            });
            continue;
        }
        let at = Point::from((surface.origin.x, surface.origin.y));
        let overlap = touches(&surface.drawn, rect).then(|| {
            let mut local = *rect;
            local.loc -= at;
            local
        });
        output_update(output, overlap, &surface.root);
    }
}

/// Every surface of a tree: the root and its subsurfaces.
fn for_each_surface(root: &WlSurface, mut each: impl FnMut(&WlSurface)) {
    with_surface_tree_downward(
        root,
        (),
        |_, _, ()| TraversalAction::DoChildren(()),
        |surface, _, ()| each(surface),
        |_, _, ()| true,
    );
}

/// Tells the tree the scale of its home output (`wl_surface.preferred_buffer_scale` for a
/// version 6 surface, the fractional preferred scale for one that asked).
fn tell_scale(
    state: &mut State,
    surface: &Placed,
    home: Option<&OutputKey>,
    outputs: &BTreeMap<OutputKey, (Output, Rectangle<i32, Logical>)>,
) {
    let Some(scale) = home
        .filter(|key| outputs.contains_key(*key))
        .and_then(|key| state.core.model.outputs().0.get(key))
        .map(|plugged| plugged.config.scale)
    else {
        return;
    };
    let mut tree = Vec::new();
    for_each_surface(&surface.root, |one| tree.push(one.clone()));
    for one in tree {
        if fractional_scale::apply_scale(&mut state.wl.fractional_scale, &one, scale).is_err() {
            tracing::debug!("a surface was destroyed before it heard its scale");
        }
    }
}

/// Enters `output` on each surface of the tree whose view touches `overlap` (the output's
/// rectangle in the root's coordinates) and leaves it on the others, on all of them when
/// `overlap` is `None`, and on those that are unmapped. After smithay's `output_update`
/// (MIT), which its desktop module does not export.
fn output_update(output: &Output, overlap: Option<Rectangle<i32, Logical>>, root: &WlSurface) {
    let view_of = |states: &smithay::wayland::compositor::SurfaceData| {
        states
            .data_map
            .get::<RendererSurfaceStateUserData>()
            .and_then(|data| data.lock().ok().and_then(|held| held.view()))
    };
    with_surface_tree_downward(
        root,
        (Point::<i32, Logical>::from((0, 0)), false),
        |_, states, (at, hidden)| match (hidden, view_of(states)) {
            (false, Some(view)) => TraversalAction::DoChildren((*at + view.offset, false)),
            _ => TraversalAction::DoChildren((*at, true)),
        },
        |surface, states, (at, hidden)| {
            let view = view_of(states).filter(|_| !hidden);
            match (overlap, view) {
                (Some(overlap), Some(view))
                    if overlap.overlaps(Rectangle::new(*at + view.offset, view.dst)) =>
                {
                    output.enter(surface);
                }
                _ => output.leave(surface),
            }
        },
        |_, _, _| true,
    );
}
