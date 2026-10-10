//! casement's motion: one ds-motion spring leg per moving quantity, retargeted from one
//! [`casement_layout::Arrangement`] to the next with position and velocity kept, and sampled at
//! a frame's instant into a [`FrameScene`] of whole sub-pixels (1/120 px).
//!
//! Floats live only inside the legs and the sampling arithmetic; [`quantise`] is the one step
//! back to integer units, so a frame is `Eq` data and a motion golden is text.
//!
//! ```
//! use casement_core::time::Mono;
//! use casement_motion::MotionBook;
//! use std::time::Duration;
//!
//! assert!(!MotionBook::empty().is_moving(Mono(Duration::ZERO)));
//! ```

mod book;
mod cause_travel;
mod frame_scene;
mod goals;
mod pose;
mod quantise;
mod retarget;
mod sample;
mod settled;
mod space_slide;
mod window_motion;

pub use book::{MotionBook, MotionKey};
pub use frame_scene::{
    CornerRadius, FrameScene, OutputFrame, PlacedWindow, ShadowLevel, StripOffset,
};
pub use quantise::quantise;
pub use retarget::retarget;
pub use sample::{CommittedSizes, WindowLook, WindowLooks, sample};
pub use settled::Settled;
