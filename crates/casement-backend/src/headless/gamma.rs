//! Gamma on virtual outputs: a ramp a client sets is remembered so a test reads it back; no
//! pixel is changed.

use super::HeadlessBackend;
use casement_core::OutputName;
use casement_core::output::GammaRamp;

/// How many steps per channel a virtual output's gamma table has.
pub const GAMMA_SIZE: usize = 256;

impl HeadlessBackend {
    /// Records `ramp` as `name`'s gamma (an output that is not plugged records nothing).
    ///
    /// ```
    /// use casement_backend::headless::HeadlessBackend;
    /// use casement_core::OutputName;
    /// use casement_core::output::GammaRamp;
    ///
    /// # fn demo(backend: &mut HeadlessBackend, name: &OutputName) {
    /// backend.set_gamma(name, GammaRamp::Identity);
    /// assert!(backend.gamma_of(name).is_some());
    /// # }
    /// ```
    pub fn set_gamma(&mut self, name: &OutputName, ramp: GammaRamp) {
        if let Some(output) = self.outputs.get_mut(name) {
            output.gamma = ramp;
        }
    }

    /// The gamma ramp last set on `name` (none when it is not plugged).
    pub fn gamma_of(&self, name: &OutputName) -> Option<&GammaRamp> {
        self.outputs.get(name).map(|output| &output.gamma)
    }
}
