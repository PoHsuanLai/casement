//! An output's transform (`wl_output.transform`), in our own words.

use serde::{Deserialize, Serialize};

/// How an output's content is rotated (counter-clockwise) and flipped before scanout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Transform {
    /// As drawn.
    Normal,
    /// Rotated 90 degrees.
    R90,
    /// Rotated 180 degrees.
    R180,
    /// Rotated 270 degrees.
    R270,
    /// Flipped about the vertical axis.
    Flipped,
    /// Flipped, then rotated 90 degrees.
    Flipped90,
    /// Flipped, then rotated 180 degrees.
    Flipped180,
    /// Flipped, then rotated 270 degrees.
    Flipped270,
}
