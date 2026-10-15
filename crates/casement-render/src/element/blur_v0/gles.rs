//! Blur on GLES: the element's half of the dual-Kawase pass (`pass::blur_kawase`). The capture
//! blurs the framebuffer under the element into the element's cache; the draw lays the result
//! over the region, or the stand-in before any capture.

use super::BlurV0Element;
use crate::element::BlurFinish;
use crate::pass::{Finished, capture_blur, finish_blur};
use smithay::backend::renderer::gles::{GlesError, GlesFrame};
use smithay::utils::user_data::UserDataMap;
use smithay::utils::{Physical, Rectangle};

/// Blurs what lies under `dst` into `cache`; a solid blur captures nothing.
pub(super) fn capture(
    frame: &mut GlesFrame<'_, '_>,
    blur: &BlurV0Element,
    dst: Rectangle<i32, Physical>,
    cache: &UserDataMap,
) -> Result<(), GlesError> {
    match blur.finish() {
        BlurFinish::Frosted => capture_blur(frame, dst, blur.plan(), cache),
        BlurFinish::Solid(_) => Ok(()),
    }
}

/// Draws the blurred copy over `blur`'s region where `damage` (relative to `dst`) asks; the
/// stand-in when nothing was captured yet, and always when the blur is solid.
pub(super) fn draw(
    frame: &mut GlesFrame<'_, '_>,
    blur: &BlurV0Element,
    dst: Rectangle<i32, Physical>,
    damage: &[Rectangle<i32, Physical>],
    cache: Option<&UserDataMap>,
) -> Result<(), GlesError> {
    if blur.finish() != BlurFinish::Frosted {
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
