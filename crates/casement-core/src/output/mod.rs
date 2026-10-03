//! Outputs: what the hardware reports (detected, never assumed, 03 M7), how each is
//! configured, and the policies that pick a first scale, a mode and a place.

mod arrangement;
mod config;
mod facts;
mod mode_pick;
mod scale_policy;

pub use arrangement::place_outputs;
pub use config::{OutputConfig, OutputPower, SavedOutputs};
pub use facts::{
    EdidFacts, EdidSerial, Mode, ModePreference, OutputFacts, PhysicalMm, VrrCapability,
};
pub use mode_pick::{ModeWish, pick_mode};
pub use scale_policy::{ScalePolicy, first_sight_scale};
