//! `zwlr_gamma_control_manager_v1`: a client (night light) drives an output's colour curve.
//! P1-Z declares it; P1-M1 creates the global and fills the functions. The ramp becomes
//! `Event::GammaSet`, the model answers with `Effect::SetGamma`, and `casement::exec` hands it
//! to the backend; destroying the control restores the identity curve.

use crate::states::{GlobalFilter, WaylandFault};
use casement_core::output::GammaRamp;
use casement_core::{Event, OutputKey};
use smithay::reexports::wayland_server::DisplayHandle;
use std::collections::BTreeMap;

/// How many steps per channel an output's gamma table has (`gamma_size`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct GammaSize(pub u32);

/// `zwlr_gamma_control_manager_v1` and the outputs under a client's control.
#[derive(Debug, Default)]
pub struct GammaPart {
    /// Each controlled output and the table size it announced.
    pub controlled: BTreeMap<OutputKey, GammaSize>,
}

impl GammaPart {
    /// Creates the global (version 1), filtered by the I7 table. P1-Z: creates nothing.
    pub fn new<D: 'static>(display: &DisplayHandle, role_filter: GlobalFilter) -> GammaPart {
        let _ = (display, role_filter, std::marker::PhantomData::<D>);
        GammaPart::default()
    }
}

/// A client's `set_gamma` on `output`: one `Event::GammaSet`.
///
/// ```no_run
/// use casement_core::output::GammaRamp;
/// use casement_wayland::gamma::{GammaPart, on_set_gamma};
///
/// # fn demo(key: casement_core::OutputKey) {
/// let mut part = GammaPart::default();
/// let events = on_set_gamma(&mut part, key, GammaRamp::Identity);
/// assert_eq!(events.len(), 1);
/// # }
/// ```
pub fn on_set_gamma(part: &mut GammaPart, output: OutputKey, ramp: GammaRamp) -> Vec<Event> {
    let _ = (part, output, ramp);
    todo!("P1-M1: gamma::on_set_gamma")
}

/// The client destroyed its control (or went away): the identity curve is restored.
pub fn on_released(part: &mut GammaPart, output: &OutputKey) -> Vec<Event> {
    let _ = (part, output);
    todo!("P1-M1: gamma::on_released")
}

/// Announces `output`'s table size to the control (`gamma_size`).
pub fn apply_size(
    part: &mut GammaPart,
    output: &OutputKey,
    size: GammaSize,
) -> Result<(), WaylandFault> {
    let _ = (part, output, size);
    todo!("P1-M1: gamma::apply_size")
}
