//! The stored form of a [`Scale`]: its 120ths as a bare number. ds-core's `Scale` has no serde
//! of its own (FINDINGS: interface ask), so stored types name this with `#[serde(with)]`.

use ds_core::geometry::scale::Scale;
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serializer};

/// Writes `scale` as its numerator.
pub(crate) fn serialize<S: Serializer>(scale: &Scale, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_u32(scale.0)
}

/// The smallest stored scale, in 120ths: a quarter (CASC-12). A file with less would show a
/// desktop of a few thousand points on one display.
pub const MIN_STORED_SCALE: u32 = 30;

/// The largest stored scale, in 120ths: eight times. Past it a window's size in points would be
/// a few pixels.
pub const MAX_STORED_SCALE: u32 = 960;

/// Reads a numerator back into a scale; a value outside [`MIN_STORED_SCALE`] to
/// [`MAX_STORED_SCALE`] is refused, as a file that is not ours.
pub(crate) fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Scale, D::Error> {
    let numerator = u32::deserialize(deserializer)?;
    match (MIN_STORED_SCALE..=MAX_STORED_SCALE).contains(&numerator) {
        true => Ok(Scale(numerator)),
        false => Err(D::Error::custom(format!(
            "a scale of {numerator}/120 is outside {MIN_STORED_SCALE} to {MAX_STORED_SCALE}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Deserialize, PartialEq)]
    struct Held(#[serde(with = "self")] Scale);

    #[test]
    fn a_stored_scale_outside_a_quarter_to_eight_times_is_refused() {
        for (numerator, accepted) in [
            (0, false),
            (29, false),
            (30, true),
            (120, true),
            (960, true),
            (961, false),
            (u32::MAX, false),
        ] {
            let read = serde_json::from_str::<Held>(&numerator.to_string());
            assert_eq!(read.is_ok(), accepted, "{numerator}");
        }
    }
}
