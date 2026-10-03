//! Geometry: logical and physical rectangles, sub-pixel units (1/120 px), output transforms,
//! and the one conversion from logical to physical at a [`ds_core::geometry::scale::Scale`].

mod convert;
mod logical;
mod physical;
pub(crate) mod scale_serde;
mod sub;
mod transform;

pub use convert::{buffer_size, to_physical};
pub use logical::{LogicalPoint, LogicalRect, LogicalSize};
pub use physical::{PhysicalRect, PhysicalSize};
pub use sub::{SubPoint, SubPx, SubSize};
pub use transform::Transform;
