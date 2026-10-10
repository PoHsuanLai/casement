//! `zwlr_gamma_control_manager_v1` (P1-M1): `casement_wayland::gamma` serves the protocol; this
//! file gives it the root's part, the output lookup, the backend's table size and the loop.
//! A set ramp is an `Event::GammaSet`; the model's `Effect::SetGamma` reaches the backend in
//! `exec::gamma`.

use super::layer_shell::output_key_of;
use crate::core_loop;
use crate::state::State;
use casement_backend::BackendKind;
use casement_backend::headless::GAMMA_SIZE;
use casement_core::{Event, OutputKey};
use casement_wayland::gamma::{GammaHandler, GammaPart, GammaSize};
use smithay::reexports::wayland_server::protocol::wl_output::WlOutput;

impl GammaHandler for State {
    fn gamma_part(&mut self) -> &mut GammaPart {
        &mut self.wl.gamma
    }

    fn gamma_output(&self, output: &WlOutput) -> Option<OutputKey> {
        output_key_of(self, output)
    }

    /// The headless backend takes 256 steps; the KMS device reports no size yet (FINDINGS,
    /// p1-m1), so a tty output answers the same; the winit window has no gamma to drive.
    fn gamma_table_size(&self, output: &OutputKey) -> Option<GammaSize> {
        self.core.model.outputs().0.get(output)?;
        match &self.backend {
            BackendKind::Headless(_) | BackendKind::Tty(_) => {
                u32::try_from(GAMMA_SIZE).ok().map(GammaSize)
            }
            _ => None,
        }
    }

    fn gamma_events(&mut self, events: Vec<Event>) {
        core_loop::feed(self, events);
    }
}
