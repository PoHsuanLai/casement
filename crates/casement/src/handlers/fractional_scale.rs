//! `wp_fractional_scale_manager_v1` (P0-H1): `new_fractional_scale` sends the preferred scale
//! before the first configure.

use crate::core_loop;
use crate::state::State;
use casement_wayland::fractional_scale;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::wayland::fractional_scale::FractionalScaleHandler;

impl FractionalScaleHandler for State {
    /// The surface already carries the primary output's scale from `new_surface`; this
    /// answers again with the scale as it is now (smithay sends only a change), so an output
    /// that changed scale between the surface and its scale object is not missed.
    fn new_fractional_scale(&mut self, surface: WlSurface) {
        let events = fractional_scale::on_preferred(&mut self.wl.fractional_scale, &surface);
        if let Some(scale) = fractional_scale::home_scale(self.core.model.outputs())
            && fractional_scale::apply_scale(&mut self.wl.fractional_scale, &surface, scale)
                .is_err()
        {
            tracing::debug!("a surface was destroyed before its scale object");
        }
        if !events.is_empty() {
            core_loop::feed(self, events);
        }
    }
}
