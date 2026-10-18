//! `zwlr_foreign_toplevel_manager_v1` (P1-M2): callbacks that feed `Event::WlrToplevel`.

use crate::state::State;
use casement_core::geometry::{LogicalPoint, LogicalSize};
use casement_wayland::wlr_toplevel::{WlrToplevelHandler, WlrToplevelPart};
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;

impl WlrToplevelHandler for State {
    fn wlr_toplevel_part(&mut self) -> &mut WlrToplevelPart {
        &mut self.wl.wlr_toplevel
    }

    /// Only a layer surface (a dock's tile) has a place casement chose: where it is arranged.
    fn surface_origin(&self, surface: &WlSurface) -> Option<LogicalPoint> {
        let layer = self.wl.layer_shell.key_of(surface)?;
        let model = &self.core.model;
        let spec = model.layers().get(&layer)?;
        let full = model.outputs().0.get(&spec.output)?.config.logical_rect();
        let committed = self
            .render
            .textures
            .layer(&layer)
            .map_or(LogicalSize::ZERO, |entry| entry.committed);
        model
            .layers()
            .arrange(&spec.output, full)
            .place(&layer, committed)
            .map(|rect| rect.origin)
    }
}
