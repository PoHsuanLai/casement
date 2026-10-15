//! Blur on GLES: the element's half of the dual-Kawase pass (`pass::blur_kawase`). The capture
//! blurs the framebuffer under the element into the element's cache; the draw lays the result
//! over the region, or the stand-in before any capture.

use super::BlurV0Element;
use crate::pass::{Finished, capture_blur, finish_blur};
use casement_core::settings::Transparency;
use smithay::backend::renderer::gles::{GlesError, GlesFrame};
use smithay::utils::user_data::UserDataMap;
use smithay::utils::{Physical, Rectangle};

/// Blurs what lies under `dst` into `cache`; a reduced blur captures nothing.
pub(super) fn capture(
    frame: &mut GlesFrame<'_, '_>,
    blur: &BlurV0Element,
    dst: Rectangle<i32, Physical>,
    cache: &UserDataMap,
) -> Result<(), GlesError> {
    match blur.transparency() {
        Transparency::Full => capture_blur(frame, dst, blur.plan(), cache),
        Transparency::Reduced => Ok(()),
    }
}

/// Draws the blurred copy over `blur`'s region where `damage` (relative to `dst`) asks; the
/// stand-in when nothing was captured yet, and always when the blur is reduced.
pub(super) fn draw(
    frame: &mut GlesFrame<'_, '_>,
    blur: &BlurV0Element,
    dst: Rectangle<i32, Physical>,
    damage: &[Rectangle<i32, Physical>],
    cache: Option<&UserDataMap>,
) -> Result<(), GlesError> {
    if blur.transparency() == Transparency::Reduced {
        return blur.draw_stand_in(frame, dst, damage);
    }
    let clipped = blur.clip(damage);
    if clipped.is_empty() {
        return Ok(());
    }
    match finish_blur(frame, blur.plan(), blur.alpha_of(), (dst, &clipped), cache)? {
        Finished::Drawn => Ok(()),
        Finished::Missing => blur.draw_stand_in(frame, dst, damage),
    }
}
