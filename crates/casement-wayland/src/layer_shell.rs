//! `zwlr_layer_shell_v1`: the shell's bars, docks, panels and wallpaper. An `Exclusive` layer
//! takes the keyboard on the commit that maps it, with the current modifiers (A16, SP1 A2); a
//! layer mapping under a resting pointer gets `enter` at once (SP1 A1).

use crate::states::{GlobalFilter, WaylandFault};
use casement_core::geometry::LogicalSize;
use casement_core::layer::LayerLevel;
use casement_core::policy::GlobalKind;
use casement_core::{Event, LayerKey, OutputKey};
use smithay::reexports::wayland_protocols_wlr::layer_shell::v1::server::zwlr_layer_shell_v1::ZwlrLayerShellV1;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::{DisplayHandle, GlobalDispatch};
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
        }
    }
}

/// A layer surface's namespace, as its client gave it (`"bar"`, `"dock"`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LayerNamespace(pub String);

/// A layer surface was created on `output` (or the output the model picks when `None`) and
/// given the key `layer`; it enters the model when it maps.
pub fn on_new_layer(
    part: &mut LayerShellPart,
    surface: &LayerSurface,
    output: Option<OutputKey>,
    level: LayerLevel,
    namespace: LayerNamespace,
    layer: LayerKey,
) -> Vec<Event> {
    let _ = (part, surface, output, level, namespace, layer);
    todo!("P0-H3: layer_shell::on_new_layer")
}

/// A layer surface committed: its initial configure, its mapping (`Event::LayerMapped`) or a
/// changed spec (`Event::LayerChanged`).
pub fn on_layer_commit(part: &mut LayerShellPart, surface: &WlSurface) -> Vec<Event> {
    let _ = (part, surface);
    todo!("P0-H3: layer_shell::on_layer_commit")
}

/// A layer surface was destroyed.
pub fn on_layer_destroyed(part: &mut LayerShellPart, surface: &LayerSurface) -> Vec<Event> {
    let _ = (part, surface);
    todo!("P0-H3: layer_shell::on_layer_destroyed")
}

/// A layer surface opened a popup.
pub fn on_layer_popup(
    part: &mut LayerShellPart,
    parent: &LayerSurface,
    popup: &PopupSurface,
) -> Vec<Event> {
    let _ = (part, parent, popup);
    todo!("P0-H3: layer_shell::on_layer_popup")
}

/// Sends `layer` its size.
pub fn apply_layer_configure(
    part: &mut LayerShellPart,
    layer: LayerKey,
    size: LogicalSize,
) -> Result<(), WaylandFault> {
    let _ = (part, layer, size);
    todo!("P0-H3: layer_shell::apply_layer_configure")
}
