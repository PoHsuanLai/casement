//! The dual-Kawase behind-surface blur (P3-FX, replacing blur v0's single downsample).
//!
//! A [`BlurLook`] says what a material asks of the blur in logical pixels (the sigma of the
//! Gaussian it should look like, the saturation of what shows through, an optional tint and
//! grain). [`BlurLook::plan`] turns it into a [`KawasePlan`] in physical pixels: how many halving
//! levels to go down and the tap offset, from the paper's relation that the spread doubles with
//! each level. `plan` is pure; the GLES passes (`gles`) run it with the shaders in
//! `src/shaders/kawase_*.frag`. Renderers without shaders keep the element's solid stand-in.

mod gles;
mod plan;

pub(crate) use gles::{capture, finish};
pub use plan::{BlurLook, KawasePlan, level_size};
