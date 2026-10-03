//! One Space's thumbnail in Mission Control's Space row.

use casement_core::OutputKey;
use casement_core::geometry::LogicalRect;
use porter_core::SpaceId;

/// Where a Space's thumbnail sits in its output's Space row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpaceThumb {
    /// The Space.
    pub space: SpaceId,
    /// The output whose row it is in.
    pub output: OutputKey,
    /// The thumbnail, in global logical coordinates.
    pub rect: LogicalRect,
}
