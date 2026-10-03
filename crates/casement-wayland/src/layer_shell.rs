//! `zwlr_layer_shell_v1`: the shell's bars, docks, panels and wallpaper. An `Exclusive` layer
//! takes the keyboard on the commit that maps it, with the current modifiers (A16, SP1 A2); a
//! layer mapping under a resting pointer gets `enter` at once (SP1 A1).
//!
//! A layer surface enters the model on its first commit (the one that asks for the initial
//! configure): the model answers with `Effect::ConfigureLayer` and, for an exclusive layer, the
//! keyboard effect, so both happen on that commit. Later commits that change what it asks for
//! are `LayerChanged`; a commit that removes the buffer, or its destruction, is
//! `LayerUnmapped`.

mod spec;

pub use spec::{anchors_of, exclusive_of, keyboard_of, layer_area, level_of, margins_of, spec_of};

use crate::states::{GlobalFilter, WaylandFault};
use casement_core::geometry::LogicalSize;
use casement_core::layer::{LayerLevel, LayerSpec};
use casement_core::policy::GlobalKind;
use casement_core::{Event, LayerKey, OutputKey};
use smithay::reexports::wayland_protocols_wlr::layer_shell::v1::server::zwlr_layer_shell_v1::ZwlrLayerShellV1;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::{DisplayHandle, GlobalDispatch};
use smithay::utils::{Logical, Size};
use smithay::wayland::shell::wlr_layer::{
    LayerSurface, WlrLayerShellGlobalData, WlrLayerShellState,
};
use smithay::wayland::shell::xdg::PopupSurface;
use std::collections::BTreeMap;

/// `zwlr_layer_shell_v1` and its surfaces.
#[derive(Debug)]
pub struct LayerShellPart {
    /// smithay's state.
    pub state: WlrLayerShellState,
    /// Every live layer surface.
    pub layers: BTreeMap<LayerKey, LayerSurface>,
    /// What the model has been told about each one.
    pub records: BTreeMap<LayerKey, LayerRecord>,
}

/// What the part remembers about one layer surface beyond smithay's handle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerRecord {
    /// The output it was created for; `None` when no output could be picked.
    pub output: Option<OutputKey>,
    /// The namespace its client gave (`"bar"`, `"dock"`).
    pub namespace: LayerNamespace,
    /// What the model knows of it.
    pub known: Known,
    /// Whether its last commit carried a buffer.
    pub content: Content,
}

/// What the model has been told about a layer surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Known {
    /// Nothing: it has not committed yet, or it unmapped.
    Nothing,
    /// It was mapped, or last changed, to this.
    As(LayerSpec),
}

/// Whether a layer surface shows a buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Content {
    /// Its last commit removed or never attached one.
    Empty,
    /// It has a buffer.
    Shown,
}

impl LayerShellPart {
    /// Creates the global (version 5), filtered by the I7 table (not for sandboxed clients).
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> LayerShellPart
    where
        D: GlobalDispatch<ZwlrLayerShellV1, WlrLayerShellGlobalData> + 'static,
    {
        LayerShellPart {
            state: WlrLayerShellState::new_with_filter::<D, _>(
                display,
                role_filter.for_global(GlobalKind::LayerShell),
            ),
            layers: BTreeMap::new(),
            records: BTreeMap::new(),
        }
    }

    /// The key of the layer whose `wl_surface` is `surface`.
    pub fn key_of(&self, surface: &WlSurface) -> Option<LayerKey> {
        self.layers
            .iter()
            .find(|(_, layer)| layer.wl_surface() == surface)
            .map(|(key, _)| *key)
    }

    /// The `wl_surface` of `layer` (the keyboard and pointer focus target, and what the
    /// renderer draws).
    pub fn surface_of(&self, layer: LayerKey) -> Option<&WlSurface> {
        self.layers.get(&layer).map(LayerSurface::wl_surface)
    }
}

/// A layer surface's namespace, as its client gave it (`"bar"`, `"dock"`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LayerNamespace(pub String);

/// A layer surface was created on `output` (the handler picks one when the client named
/// none) and given the key `layer`; it enters the model on its first commit.
pub fn on_new_layer(
    part: &mut LayerShellPart,
    surface: &LayerSurface,
    output: Option<OutputKey>,
    level: LayerLevel,
    namespace: LayerNamespace,
    layer: LayerKey,
) -> Vec<Event> {
    // The level is also in the committed state, which wins from the first commit on.
    let _ = level;
    part.layers.insert(layer, surface.clone());
    part.records.insert(
        layer,
        LayerRecord {
            output,
            namespace,
            known: Known::Nothing,
            content: Content::Empty,
        },
    );
    Vec::new()
}

/// A layer surface committed: its initial configure (it maps into the model and the model
/// answers with its size), a changed spec (`Event::LayerChanged`), or its buffer going away
/// (`Event::LayerUnmapped`).
pub fn on_layer_commit(part: &mut LayerShellPart, surface: &WlSurface) -> Vec<Event> {
    let Some(layer) = part.key_of(surface) else {
        return Vec::new();
    };
    let (Some(handle), Some(record)) = (part.layers.get(&layer), part.records.get_mut(&layer))
    else {
        return Vec::new();
    };
    let (cached, content) = handle.with_cached_state(|cached| {
        let content = match cached.last_acked {
            Some(_) => Content::Shown,
            None => Content::Empty,
        };
        (*cached, content)
    });
    let was = std::mem::replace(&mut record.content, content);
    let Some(output) = record.output.clone() else {
        return Vec::new();
    };
    let spec = spec_of(&output, &cached, &record.namespace.0);
    let known = std::mem::replace(&mut record.known, Known::Nothing);
    let (known, events) = after_commit(known, was, content, spec, layer);
    record.known = known;
    events
}

/// What a commit tells the model, and what the model then knows: nothing yet means the layer
/// maps now (the model answers with its configure); a buffer going away unmaps it; otherwise a
/// spec that differs from what the model knows is a change.
fn after_commit(
    known: Known,
    was: Content,
    now: Content,
    spec: LayerSpec,
    layer: LayerKey,
) -> (Known, Vec<Event>) {
    match (known, was, now) {
        (Known::As(_), Content::Shown, Content::Empty) => {
            (Known::Nothing, vec![Event::LayerUnmapped { layer }])
        }
        (Known::Nothing, _, _) => (
            Known::As(spec.clone()),
            vec![Event::LayerMapped { layer, spec }],
        ),
        (Known::As(held), _, _) if held != spec => (
            Known::As(spec.clone()),
            vec![Event::LayerChanged { layer, spec }],
        ),
        (held @ Known::As(_), _, _) => (held, Vec::new()),
    }
}

/// A layer surface was destroyed.
pub fn on_layer_destroyed(part: &mut LayerShellPart, surface: &LayerSurface) -> Vec<Event> {
    let Some(layer) = part.key_of(surface.wl_surface()) else {
        return Vec::new();
    };
    part.layers.remove(&layer);
    match part.records.remove(&layer).map(|record| record.known) {
        Some(Known::As(_)) => vec![Event::LayerUnmapped { layer }],
        Some(Known::Nothing) | None => Vec::new(),
    }
}

/// A layer surface opened a popup. A popup hangs off its layer's surface tree and takes no part
/// in the model at P0, so nothing is reported; the handler tracks it with the popup manager.
pub fn on_layer_popup(
    part: &mut LayerShellPart,
    parent: &LayerSurface,
    popup: &PopupSurface,
) -> Vec<Event> {
    let _ = popup;
    debug_assert!(
        part.key_of(parent.wl_surface()).is_some(),
        "a popup's layer parent is a live layer surface"
    );
    Vec::new()
}

/// Sends `layer` its size (the initial configure when it has none yet, else a configure only
/// when the size changed).
pub fn apply_layer_configure(
    part: &mut LayerShellPart,
    layer: LayerKey,
    size: LogicalSize,
) -> Result<(), WaylandFault> {
    let surface = part.layers.get(&layer).ok_or(WaylandFault::SurfaceGone)?;
    if !surface.alive() {
        return Err(WaylandFault::SurfaceGone);
    }
    let wire: Size<i32, Logical> = crate::convert::size_to(size);
    surface.with_pending_state(|state| state.size = Some(wire));
    surface.send_pending_configure();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use casement_core::OutputName;
    use casement_core::layer::{
        Anchor, Anchors, ExclusiveZone, KeyboardInteractivity, Margins, ShellRoleTag,
    };
    use casement_core::output::{EdidFacts, EdidSerial};

    fn spec(margin_top: i32) -> LayerSpec {
        let edid = EdidFacts {
            make: String::new(),
            model: String::new(),
            serial: EdidSerial::Unknown,
        };
        LayerSpec {
            output: OutputKey::of(&edid, &OutputName::parse("HEADLESS-1").expect("name")),
            level: LayerLevel::Top,
            anchor: Anchors {
                top: Anchor::Held,
                bottom: Anchor::Free,
                left: Anchor::Held,
                right: Anchor::Held,
            },
            exclusive: ExclusiveZone::Reserve(30),
            margin: Margins {
                top: margin_top,
                right: 0,
                bottom: 0,
                left: 0,
            },
            keyboard: KeyboardInteractivity::Exclusive,
            namespace: "bar".to_owned(),
            role: ShellRoleTag::Unassigned,
        }
    }

    const LAYER: LayerKey = LayerKey(3);

    #[test]
    fn the_first_commit_maps_a_layer_and_a_buffer_changes_nothing() {
        let (known, events) = after_commit(
            Known::Nothing,
            Content::Empty,
            Content::Empty,
            spec(0),
            LAYER,
        );
        assert_eq!(
            events,
            vec![Event::LayerMapped {
                layer: LAYER,
                spec: spec(0)
            }]
        );
        assert_eq!(known, Known::As(spec(0)));
        // The commit that attaches the first buffer asks for the same thing.
        let (known, events) = after_commit(known, Content::Empty, Content::Shown, spec(0), LAYER);
        assert_eq!(events, Vec::new());
        assert_eq!(known, Known::As(spec(0)));
    }

    #[test]
    fn a_changed_ask_is_a_change_and_an_unchanged_one_is_silent() {
        let (known, events) = after_commit(
            Known::As(spec(0)),
            Content::Shown,
            Content::Shown,
            spec(8),
            LAYER,
        );
        assert_eq!(
            events,
            vec![Event::LayerChanged {
                layer: LAYER,
                spec: spec(8)
            }]
        );
        assert_eq!(known, Known::As(spec(8)));
        let (_, events) = after_commit(known, Content::Shown, Content::Shown, spec(8), LAYER);
        assert_eq!(events, Vec::new());
    }

    #[test]
    fn removing_the_buffer_unmaps_and_the_next_commit_maps_again() {
        let (known, events) = after_commit(
            Known::As(spec(0)),
            Content::Shown,
            Content::Empty,
            spec(0),
            LAYER,
        );
        assert_eq!(events, vec![Event::LayerUnmapped { layer: LAYER }]);
        assert_eq!(known, Known::Nothing);
        let (_, events) = after_commit(known, Content::Empty, Content::Empty, spec(0), LAYER);
        assert!(matches!(events.as_slice(), [Event::LayerMapped { .. }]));
    }
}
