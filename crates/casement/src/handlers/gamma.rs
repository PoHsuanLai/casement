//! `zwlr_gamma_control_manager_v1` (P1-M1): `casement_wayland::gamma` serves the protocol; this
//! file gives it the root's part, the output lookup, the backend's table size and the loop.
//! A set ramp is an `Event::GammaSet`; the model's `Effect::SetGamma` reaches the backend in
//! `exec::gamma`.

use crate::state::State;
use casement_core::OutputKey;
use casement_wayland::gamma::{GammaHandler, GammaPart, GammaSize};

impl GammaHandler for State {
    fn gamma_part(&mut self) -> &mut GammaPart {
        &mut self.wl.gamma
    }

    /// The headless backend takes 256 steps; a tty output has as many as its CRTC reports
    /// (`GAMMA_LUT_SIZE`; none for a CRTC with no table); the winit window has no gamma to drive.
    fn gamma_table_size(&self, output: &OutputKey) -> Option<GammaSize> {
        let name = &self.core.model.outputs().0.get(output)?.facts.name;
        self.backend.driver().gamma_steps(name).map(GammaSize)
    }
}
