//! casement's settings as typed values. The stored document and its schema live in the
//! composition root (`casement/src/config/`, ds-settings); this is what it converts into.

mod bindings;
mod gpu;
mod keyboard;
mod outputs;
mod pointer;
mod spaces;
mod windows;

pub use gpu::GpuSettings;
pub use keyboard::{KeyboardSettings, RepeatRate, XkbNames};
pub use outputs::OutputSettings;
pub use pointer::{AccelProfile, PointerProfile, PointerSettings};
pub use spaces::SpaceSettings;
pub use windows::{Enabled, FirstClick, MinimiseEffect, TitleDoubleClick, WindowSettings};

use crate::input::Bindings;

/// Every setting casement reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CasementSettings {
    /// Window management (03 §1.17).
    pub windows: WindowSettings,
    /// Spaces.
    pub spaces: SpaceSettings,
    /// The keyboard.
    pub keyboard: KeyboardSettings,
    /// Key bindings.
    pub bindings: Bindings,
    /// Outputs.
    pub outputs: OutputSettings,
    /// Mice and touchpads.
    pub pointer: PointerSettings,
    /// Which GPU renders.
    pub gpu: GpuSettings,
}

impl CasementSettings {
    /// The defaults: 03 §1.17's Mac defaults, a 400 ms key-repeat delay at 10 per second, the
    /// system's XKB defaults, and the Mac's first-sight scale.
    pub fn defaults() -> CasementSettings {
        CasementSettings {
            windows: WindowSettings::defaults(),
            spaces: SpaceSettings::defaults(),
            keyboard: KeyboardSettings::defaults(),
            bindings: bindings::defaults(),
            outputs: OutputSettings::defaults(),
            pointer: PointerSettings::defaults(),
            gpu: GpuSettings::defaults(),
        }
    }
}

#[cfg(test)]
mod tests;
