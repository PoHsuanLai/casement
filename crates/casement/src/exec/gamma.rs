//! Gamma (P1-M1): `Effect::SetGamma` reaches the backend's KMS device.

use crate::state::State;
use casement_backend::{BackendKind, TtyBackend};
use casement_core::output::GammaRamp;
use casement_core::{OutputKey, OutputName};

/// Drives `output` with `ramp`: the tty backend loads it on the output's CRTC; the headless
/// backend remembers it so a test reads it back. A failure is logged and degrades.
pub(super) fn set_gamma(state: &mut State, output: OutputKey, ramp: GammaRamp) {
    let Some(name) = name_of(state, &output) else {
        return;
    };
    match &mut state.backend {
        BackendKind::Headless(backend) => backend.set_gamma(&name, ramp),
        BackendKind::Tty(tty) => load(tty, &name, &ramp),
        _ => tracing::debug!(output = %name.as_str(), "this backend has no gamma to drive"),
    }
}

/// The connector name of a plugged output.
fn name_of(state: &State, output: &OutputKey) -> Option<OutputName> {
    state
        .core
        .model
        .outputs()
        .0
        .get(output)
        .map(|plugged| plugged.facts.name.clone())
}

/// Loads `ramp` on the CRTC of the connector `name` through the device that drives it.
fn load(tty: &mut TtyBackend, name: &OutputName, ramp: &GammaRamp) {
    let Some(kms) = tty.devices.driving(name) else {
        tracing::warn!(output = %name.as_str(), "no display device drives this output");
        return;
    };
    let connector = kms
        .resources()
        .connectors
        .iter()
        .find(|facts| facts.name == *name)
        .map(|facts| facts.id);
    let Some(connector) = connector else {
        tracing::warn!(output = %name.as_str(), "the device has no such connector");
        return;
    };
    if let Err(error) = kms.set_gamma(connector, ramp) {
        tracing::warn!(output = %name.as_str(), %error, "the gamma ramp was not loaded");
    }
}
