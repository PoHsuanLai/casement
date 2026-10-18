//! What the nested window makes of its output: the pure parts of the winit backend. The window's
//! physical size and the host's scale factor are the output's mode and scale (the host is a
//! compositor too, and it decides both), and the window's framebuffer is drawn flipped.

use crate::headless::VirtualOutputSpec;
use casement_core::geometry::PhysicalSize;
use casement_core::output::Mode;
use ds_core::geometry::scale::Scale;
use smithay::backend::renderer::gles::GlesRenderer;
use smithay::backend::winit::WinitGraphicsBackend;
use smithay::utils::{Physical, Size, Transform};

/// The transform the damage tracker draws a winit window with: smithay's GLES framebuffer on the
/// window's surface is y-flipped, and the flip undoes it (what anvil's winit backend does). It is
/// the window's own: the output the model and the clients see stays `Normal`, so a capture of
/// it is upright and never flipped twice.
pub const HOST_TRANSFORM: Transform = Transform::Flipped180;

/// Scale 1 in 120ths: what a host factor that is not a positive number means.
const UNIT_SCALE: Scale = Scale(120);

/// `base` with the window's mode size and scale: `size` physical pixels at the host's
/// `scale_factor` (a 2880x1800 window on a 2x host is a 2880x1800 mode at 240/120).
///
/// ```
/// use casement_backend::{window_spec, VirtualOutputSpec};
/// use casement_core::geometry::PhysicalSize;
///
/// let base = VirtualOutputSpec::parse("WINIT-1:1440x900@60000:120:0,0").unwrap();
/// let spec = window_spec(&base, PhysicalSize { w: 2880, h: 1800 }, 2.0);
/// assert_eq!((spec.mode.size.w, spec.scale.0), (2880, 240));
/// ```
pub fn window_spec(
    base: &VirtualOutputSpec,
    size: PhysicalSize,
    scale_factor: f64,
) -> VirtualOutputSpec {
    VirtualOutputSpec {
        mode: Mode { size, ..base.mode },
        scale: scale_of(scale_factor),
        ..base.clone()
    }
}

/// The host's scale factor in 120ths, at least 1; a factor that is not a positive number is 1.
fn scale_of(factor: f64) -> Scale {
    Some(factor)
        .filter(|f| f.is_finite() && *f > 0.0)
        .map_or(UNIT_SCALE, |f| {
            Scale((f * 120.0).round().clamp(1.0, f64::from(u32::MAX)) as u32)
        })
}

/// A window's size in our words (a negative side is none).
pub(crate) fn physical_of(size: Size<i32, Physical>) -> PhysicalSize {
    let side = |n: i32| u32::try_from(n).unwrap_or(0);
    PhysicalSize {
        w: side(size.w),
        h: side(size.h),
    }
}

/// The window's current size and scale factor as a spec over `base`.
pub(crate) fn spec_of_window(
    base: &VirtualOutputSpec,
    graphics: &WinitGraphicsBackend<GlesRenderer>,
) -> VirtualOutputSpec {
    window_spec(
        base,
        physical_of(graphics.window_size()),
        graphics.scale_factor(),
    )
}
