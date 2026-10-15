//! casement's layout seam: given what the model holds ([`SceneSpec`]), where every window,
//! Space and overview slot belongs ([`Arrangement`]). Pure and time-free; motion animates
//! between arrangements, the renderer draws what motion samples.
//!
//! [`RustScene`] is the production layout: arithmetic for floating and cascading windows, and
//! Taffy (behind `taffy_bridge`, the only module naming it) where a layout really is flex or
//! grid (Split View, Exposé, from P3).
//!
//! ```
//! use casement_core::settings::CasementSettings;
//! use casement_layout::{Arrangement, LayoutKnobs, RustScene, SceneLayout, SceneSpec};
//!
//! let knobs = LayoutKnobs::of(&CasementSettings::defaults());
//! let arrangement = RustScene.arrange(&SceneSpec { outputs: Vec::new() }, &knobs);
//! assert_eq!(arrangement, Arrangement::default());
//! ```

mod arrangement;
mod frame;
mod knobs;
mod look;
mod of_model;
mod rust_scene;
mod scene_layout;
mod spec;
mod taffy_bridge;

pub use arrangement::{
    Arrangement, BackdropTarget, ConfigureNeed, DividerTarget, LayoutReport, OverviewSlot,
    SlotKind, SpaceTarget, SpaceThumb, WindowTarget,
};
pub use ds_style::appearance::theme::{Scheme, Theme};
pub use ds_style::tokens::chrome::GroupHover;
pub use frame::{frame_style, metrics as frame_metrics, pressed, scheme_of};
pub use knobs::LayoutKnobs;
pub use look::{overview_dim, overview_plate, overview_ring, split_backdrop, tile_preview_fill};
pub use of_model::{spec_of, spec_of_framed};
pub use rust_scene::{RustScene, zone_rect};
pub use scene_layout::SceneLayout;
pub use spec::{
    OutputSpec, OverviewKind, SceneMode, SceneSpec, SpaceArrangement, SpaceSpec, StackIndex,
    WindowPlacement, WindowSpec,
};
