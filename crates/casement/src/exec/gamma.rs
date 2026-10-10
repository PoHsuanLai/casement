//! Gamma (P1-Z; filled by P1-M1): `Effect::SetGamma` reaches the backend's KMS device.

use crate::state::State;
use casement_core::OutputKey;
use casement_core::output::GammaRamp;

/// Drives `output` with `ramp`: the tty backend loads it on the output's CRTC; the headless
/// backend remembers it so a test reads it back. A failure is logged and degrades.
pub(super) fn set_gamma(state: &mut State, output: OutputKey, ramp: GammaRamp) {
    let _ = (state, output, ramp);
    todo!("P1-M1: exec::gamma::set_gamma")
}
