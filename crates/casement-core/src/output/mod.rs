//! Outputs: what the hardware reports (detected, never assumed, 03 M7), how each is
//! configured, and the policies that pick a first scale, a mode and a place.

mod arrangement;
mod config;
mod density;
mod facts;
mod gamma;
mod gamut;
mod mode_pick;
mod night_light;
mod request;
mod scale_policy;
mod tag;

pub use arrangement::place_outputs;
pub use config::{OutputConfig, OutputPower, SavedOutputs};
pub use facts::{
    EdidFacts, EdidSerial, Mode, ModePreference, OutputFacts, PhysicalMm, VrrCapability,
};
pub use gamma::{GammaRamp, GammaTable};
pub use gamut::{Chromaticity, D65, DISPLAY_P3, Gamut, Primaries, SRGB};
pub use mode_pick::{ModeWish, pick_mode};
pub use night_light::{Kelvin, MinuteOfDay, NightLight, Schedule, Tint};
pub use request::{OutputChangeSet, OutputRefusal, keeps_one_lit, resolve_change};
pub use scale_policy::{ScalePolicy, first_sight_scale};
pub use tag::{ColorTag, TransferCurve};

#[cfg(test)]
mod tests;
