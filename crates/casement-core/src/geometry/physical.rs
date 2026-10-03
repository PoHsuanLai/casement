//! Physical (device-pixel) sizes and rectangles: what buffers, modes and scanout speak.

use serde::{Deserialize, Serialize};

/// A width and a height in device pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PhysicalSize {
    /// The width.
    pub w: u32,
    /// The height.
    pub h: u32,
}

/// An axis-aligned rectangle in device pixels, half-open like
/// [`super::LogicalRect`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PhysicalRect {
    /// The left edge.
    pub x: i32,
    /// The top edge.
    pub y: i32,
    /// The width.
    pub w: u32,
    /// The height.
    pub h: u32,
}
