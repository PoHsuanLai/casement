//! `zwlr_output_power_manager_v1` (P1-M1): `casement_wayland::output_power` serves the
//! protocol; this file gives it the root's part, the output lookup, the model's power and the
//! loop. A request is an `Event::OutputRequested` that changes only the power.

use super::layer_shell::output_key_of;
use crate::core_loop;
use crate::state::State;
use casement_core::output::OutputPower;
use casement_core::{Event, OutputKey};
use casement_wayland::output_power::{OutputPowerHandler, OutputPowerPart};
use smithay::reexports::wayland_server::protocol::wl_output::WlOutput;

impl OutputPowerHandler for State {
    fn output_power_part(&mut self) -> &mut OutputPowerPart {
        &mut self.wl.output_power
    }

    fn output_power_output(&self, output: &WlOutput) -> Option<OutputKey> {
        output_key_of(self, output)
    }

    fn output_power_of(&self, output: &OutputKey) -> Option<OutputPower> {
        self.core
            .model
            .outputs()
            .0
            .get(output)
            .map(|plugged| plugged.config.power)
    }

    fn output_power_events(&mut self, events: Vec<Event>) {
        core_loop::feed(self, events);
    }
}
