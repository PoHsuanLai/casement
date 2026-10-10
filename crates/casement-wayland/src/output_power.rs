//! `zwlr_output_power_manager_v1`: a client (an idle daemon, sill's lock) lights an output or
//! turns it dark. P1-Z declares it; P1-M1 creates the global and fills the functions. Works on
//! virtual outputs too: power is part of `OutputConfig`.

use crate::states::{GlobalFilter, WaylandFault};
use casement_core::output::OutputPower;
use casement_core::{Event, OutputKey};
use smithay::reexports::wayland_server::DisplayHandle;
use std::collections::BTreeSet;

/// `zwlr_output_power_manager_v1` and the outputs a client holds a control on.
#[derive(Debug, Default)]
pub struct OutputPowerPart {
    /// The outputs with a live `zwlr_output_power_v1`.
    pub controls: BTreeSet<OutputKey>,
}

impl OutputPowerPart {
    /// Creates the global (version 1), filtered by the I7 table. P1-Z: creates nothing.
    pub fn new<D: 'static>(display: &DisplayHandle, role_filter: GlobalFilter) -> OutputPowerPart {
        let _ = (display, role_filter, std::marker::PhantomData::<D>);
        OutputPowerPart::default()
    }
}

/// A client's `set_mode` on `output`: one `Event::OutputRequested` that changes only the power.
///
/// ```no_run
/// use casement_core::output::OutputPower;
/// use casement_wayland::output_power::{OutputPowerPart, on_set_mode};
///
/// # fn demo(key: casement_core::OutputKey) {
/// let mut part = OutputPowerPart::default();
/// let events = on_set_mode(&mut part, key, OutputPower::Off);
/// assert_eq!(events.len(), 1);
/// # }
/// ```
pub fn on_set_mode(
    part: &mut OutputPowerPart,
    output: OutputKey,
    power: OutputPower,
) -> Vec<Event> {
    let _ = (part, output, power);
    todo!("P1-M1: output_power::on_set_mode")
}

/// Tells the holders of a control on `output` its power is now `power` (`mode` event).
pub fn apply_mode(
    part: &mut OutputPowerPart,
    output: &OutputKey,
    power: OutputPower,
) -> Result<(), WaylandFault> {
    let _ = (part, output, power);
    todo!("P1-M1: output_power::apply_mode")
}
