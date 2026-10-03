//! One Space's origin.

use casement_core::OutputKey;
use casement_core::geometry::LogicalPoint;
use porter_core::SpaceId;

/// Where a Space's top-left sits: the shown Space at its output's usable origin, the others
/// beside it in the output's strip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpaceTarget {
    /// The Space.
    pub space: SpaceId,
    /// The output whose strip it is in.
    pub output: OutputKey,
    /// Its origin, in global logical coordinates.
    pub origin: LogicalPoint,
}
