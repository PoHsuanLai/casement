//! The stored form of a [`Scale`]: its 120ths as a bare number. ds-core's `Scale` has no serde
//! of its own (FINDINGS: interface ask), so stored types name this with `#[serde(with)]`.

use ds_core::geometry::scale::Scale;
use serde::{Deserialize, Deserializer, Serializer};

/// Writes `scale` as its numerator.
pub(crate) fn serialize<S: Serializer>(scale: &Scale, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_u32(scale.0)
}

/// Reads a numerator back into a scale.
pub(crate) fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Scale, D::Error> {
    u32::deserialize(deserializer).map(Scale)
}
