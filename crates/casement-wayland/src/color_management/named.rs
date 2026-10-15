//! The wire's names and numbers as our types, and back (the edge converts: CONVENTIONS 3).

use casement_core::output::{
    Chromaticity, D65, DISPLAY_P3, Primaries, SRGB, TransferCurve,
};
use smithay::reexports::wayland_protocols::wp::color_management::v1::server::wp_color_manager_v1::{
    Primaries as NamedPrimaries, TransferFunction,
};
use smithay::reexports::wayland_server::WEnum;

/// The primaries a client named, when casement serves that name.
pub(super) fn primaries_of_name(name: WEnum<NamedPrimaries>) -> Option<Primaries> {
    match name {
        WEnum::Value(NamedPrimaries::Srgb) => Some(SRGB),
        WEnum::Value(NamedPrimaries::DisplayP3) => Some(DISPLAY_P3),
        _ => None,
    }
}

/// The curve a client named, when casement serves that name.
pub(super) fn curve_of_name(name: WEnum<TransferFunction>) -> Option<TransferCurve> {
    match name {
        WEnum::Value(TransferFunction::Gamma22) => Some(TransferCurve::Gamma22),
        WEnum::Value(TransferFunction::ExtLinear) => Some(TransferCurve::Linear),
        WEnum::Value(TransferFunction::CompoundPower24 | TransferFunction::Srgb) => {
            Some(TransferCurve::Srgb)
        }
        _ => None,
    }
}

/// The name an output's or a surface's curve goes by on the wire for a client of `version`.
pub(super) fn name_of_curve(curve: TransferCurve, version: u32) -> TransferFunction {
    match (curve, version) {
        (TransferCurve::Gamma22, _) => TransferFunction::Gamma22,
        (TransferCurve::Linear, _) => TransferFunction::ExtLinear,
        (TransferCurve::Srgb, 1) => TransferFunction::Srgb,
        (TransferCurve::Srgb, _) => TransferFunction::CompoundPower24,
    }
}

/// A chromaticity in millionths on the wire as ten-thousandths, rounded; none outside 0..1.
fn narrow(value: i32) -> Option<u16> {
    let scaled = u16::try_from((i64::from(value) + 50).div_euclid(100)).ok()?;
    (scaled <= 10_000).then_some(scaled)
}

/// A chromaticity in ten-thousandths as millionths.
pub(super) fn widen(value: u16) -> i32 {
    i32::from(value) * 100
}

/// The primaries a client gave as eight coordinates (red, green, blue, white, x then y, in
/// millionths); the white is ignored (D65 is assumed). None when a coordinate is out of range or
/// a primary's y is zero.
pub(super) fn primaries_of_coordinates(c: [i32; 8]) -> Option<Primaries> {
    let point = |x: i32, y: i32| -> Option<Chromaticity> {
        let (x, y) = (narrow(x)?, narrow(y)?);
        (y > 0).then_some(Chromaticity { x, y })
    };
    Some(Primaries {
        red: point(c[0], c[1])?,
        green: point(c[2], c[3])?,
        blue: point(c[4], c[5])?,
    })
}

/// A primaries' eight wire coordinates with the D65 white.
pub(super) fn coordinates_of(primaries: Primaries) -> [i32; 8] {
    [
        widen(primaries.red.x),
        widen(primaries.red.y),
        widen(primaries.green.x),
        widen(primaries.green.y),
        widen(primaries.blue.x),
        widen(primaries.blue.y),
        widen(D65.x),
        widen(D65.y),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coordinates_round_trip_through_millionths() {
        assert_eq!(
            primaries_of_coordinates(coordinates_of(DISPLAY_P3)),
            Some(DISPLAY_P3)
        );
        assert_eq!(coordinates_of(SRGB)[..2], [640_000, 330_000]);
    }

    #[test]
    fn bad_coordinates_are_refused() {
        let mut bad = coordinates_of(SRGB);
        bad[0] = -1;
        assert_eq!(primaries_of_coordinates(bad), None);
        let mut flat = coordinates_of(SRGB);
        flat[3] = 0;
        assert_eq!(primaries_of_coordinates(flat), None);
        let mut far = coordinates_of(SRGB);
        far[4] = 2_000_000;
        assert_eq!(primaries_of_coordinates(far), None);
    }

    #[test]
    fn names_map_to_curves_and_primaries() {
        assert_eq!(
            curve_of_name(WEnum::Value(TransferFunction::Gamma22)),
            Some(TransferCurve::Gamma22)
        );
        assert_eq!(curve_of_name(WEnum::Value(TransferFunction::Hlg)), None);
        assert_eq!(curve_of_name(WEnum::Unknown(99)), None);
        assert_eq!(
            primaries_of_name(WEnum::Value(NamedPrimaries::DisplayP3)),
            Some(DISPLAY_P3)
        );
        assert_eq!(
            primaries_of_name(WEnum::Value(NamedPrimaries::Bt2020)),
            None
        );
        assert_eq!(
            name_of_curve(TransferCurve::Srgb, 1),
            TransferFunction::Srgb
        );
        assert_eq!(
            name_of_curve(TransferCurve::Srgb, 2),
            TransferFunction::CompoundPower24
        );
    }
}
