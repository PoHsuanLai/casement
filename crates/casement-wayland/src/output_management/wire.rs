//! The wire's integers and enums as our types, and back.

use casement_protocols::vocab::Transform;
use smithay::reexports::wayland_server::protocol::wl_output::Transform as WireTransform;

/// Our transform as `wl_output`'s.
pub(super) fn transform_to(transform: Transform) -> WireTransform {
    match transform {
        Transform::Normal => WireTransform::Normal,
        Transform::R90 => WireTransform::_90,
        Transform::R180 => WireTransform::_180,
        Transform::R270 => WireTransform::_270,
        Transform::Flipped => WireTransform::Flipped,
        Transform::Flipped90 => WireTransform::Flipped90,
        Transform::Flipped180 => WireTransform::Flipped180,
        Transform::Flipped270 => WireTransform::Flipped270,
    }
}

/// `wl_output`'s transform as ours; none for a value the enum does not have.
pub(super) fn transform_from(transform: WireTransform) -> Option<Transform> {
    match transform {
        WireTransform::Normal => Some(Transform::Normal),
        WireTransform::_90 => Some(Transform::R90),
        WireTransform::_180 => Some(Transform::R180),
        WireTransform::_270 => Some(Transform::R270),
        WireTransform::Flipped => Some(Transform::Flipped),
        WireTransform::Flipped90 => Some(Transform::Flipped90),
        WireTransform::Flipped180 => Some(Transform::Flipped180),
        WireTransform::Flipped270 => Some(Transform::Flipped270),
        _ => None,
    }
}

/// A length for a wire `int` (beyond `i32::MAX` saturates).
pub(super) fn side(value: u32) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}
