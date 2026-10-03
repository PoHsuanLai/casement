//! Layer-shell surfaces: the bar, the dock, wallpapers, overlays, and the area they leave.

mod book;
#[allow(
    clippy::module_inception,
    reason = "the freeze plan names the record file after its concept"
)]
mod layer;

pub use book::LayerBook;
pub use layer::{
    Anchor, Anchors, ExclusiveZone, KeyboardInteractivity, LayerLevel, LayerSpec, Margins,
    ShellRoleTag,
};

#[cfg(test)]
mod usable_area_tests;
