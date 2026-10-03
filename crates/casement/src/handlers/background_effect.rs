//! `ext_background_effect_manager_v1` (P0-H8): `set_blur_region` and `unset_blur_region` reach
//! `casement_wayland::background_effect::on_blur_region`; the region itself waits in smithay's
//! double-buffered cached state for the surface commit.

use crate::core_loop;
use crate::state::State;
use casement_wayland::background_effect::{on_blur_region, region_rects};
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::wayland::background_effect::ExtBackgroundEffectHandler;
use smithay::wayland::compositor::RegionAttributes;

impl ExtBackgroundEffectHandler for State {
    fn set_blur_region(&mut self, wl_surface: WlSurface, region: RegionAttributes) {
        let events = on_blur_region(
            &mut self.wl.background_effect,
            &wl_surface,
            region_rects(&region),
        );
        core_loop::feed(self, events);
    }

    fn unset_blur_region(&mut self, wl_surface: WlSurface) {
        let events = on_blur_region(&mut self.wl.background_effect, &wl_surface, Vec::new());
        core_loop::feed(self, events);
    }
}
