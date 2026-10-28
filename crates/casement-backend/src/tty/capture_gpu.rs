//! Capture through the display GPU (wave-2026-10-15/capgpu): the seam a display device lends so
//! a capture frame is drawn into a client's dma-buf by the GPU's own renderer, and the pure
//! choice between that renderer and the software one.
//!
//! The real lender is `capture_gpu_real.rs` (the device's GLES renderer, the one that draws the
//! scanout); the fake is `casement_testkit::fakes::FakeCaptureGpu`. The software renderer (the
//! capture shadow's pixman) stays the fallback: it binds linear buffers only, so a client that
//! allocated a GPU modifier can only be answered by the GPU.
//!
//! ```
//! use casement_backend::tty::{CaptureRoute, capture_route};
//! use smithay::backend::allocator::{Format, Fourcc, Modifier};
//!
//! let linear = Format { code: Fourcc::Argb8888, modifier: Modifier::Linear };
//! assert_eq!(capture_route(linear, &[linear]), CaptureRoute::Gpu);
//! assert_eq!(capture_route(linear, &[]), CaptureRoute::Software);
//! ```

use crate::backend::BackendError;
use crate::headless::FrameSource;
use casement_core::OutputName;
use casement_core::geometry::{PhysicalRect, PhysicalSize, Transform};
use casement_core::output::OutputConfig;
use casement_render::RenderError;
use casement_render::pass::CapturedFrame;
use ds_core::geometry::scale::Scale;
use smithay::backend::allocator::dmabuf::Dmabuf;
use smithay::backend::allocator::{Format, Fourcc};
use smithay::backend::renderer::Bind;
use smithay::backend::renderer::gles::GlesTexture;

/// A display device's renderer, lent to draw captures.
pub trait CaptureGpu: std::fmt::Debug {
    /// The dma-buf formats and modifiers it draws a capture into (`Argb8888` and `Xrgb8888`).
    fn formats(&self) -> Vec<Format>;

    /// Draws `frame` into the client's dma-buf `target`, and returns once the GPU is done.
    fn write_dmabuf(
        &mut self,
        target: &mut Dmabuf,
        frame: &CapturedFrame,
    ) -> Result<(), BackendError>;

    /// The device this renderer draws on (its render node's number), when it can tell. Compared
    /// with the device clients allocate on, to tell a buffer on another GPU.
    fn device(&self) -> Option<u64> {
        None
    }

    /// Whether it keeps each watched output's frame on the GPU ([`draw_scene`](Self::draw_scene)
    /// and the calls after it). Absent by default: the capture shadow reads frames back.
    fn scenes(&self) -> SceneSupport {
        SceneSupport::Absent
    }

    /// Draws `source` as the frame `output` shows, into images that stay on the GPU (one with
    /// the cursor, one without); the area that changed since the last call.
    fn draw_scene(
        &mut self,
        output: &OutputName,
        picture: &Picture,
        source: &dyn FrameSource,
    ) -> Result<Vec<PhysicalRect>, BackendError> {
        let _ = (output, picture, source);
        Err(RenderError::NoTarget.into())
    }

    /// Draws `from`, a part of the frame last drawn for `output`, into the client's dma-buf
    /// `target` on the GPU, and returns once it is done. No CPU read.
    fn blit_scene(
        &mut self,
        output: &OutputName,
        cursor: bool,
        from: PhysicalRect,
        target: &mut Dmabuf,
    ) -> Result<(), BackendError> {
        let _ = (output, cursor, from, target);
        Err(BackendError::NoFrame)
    }

    /// `area` of the frame last drawn for `output` read back to the CPU, for a shared-memory
    /// buffer: only those pixels cross to the CPU (a one-pixel colour pick reads one pixel).
    /// [`BackendError::NoFrame`] when `area` is not inside the frame.
    fn read_scene(
        &mut self,
        output: &OutputName,
        cursor: bool,
        area: PhysicalRect,
    ) -> Result<CapturedFrame, BackendError> {
        let _ = (output, cursor, area);
        Err(BackendError::NoFrame)
    }

    /// `output` is no longer watched: its images are freed.
    fn forget_scene(&mut self, output: &OutputName) {
        let _ = output;
    }

    /// Renders `source` alone straight into the client's dma-buf `target` of exactly `size` at
    /// `scale` (window capture), and returns once it is done. No CPU read.
    fn render_window(
        &mut self,
        target: &mut Dmabuf,
        size: PhysicalSize,
        scale: Scale,
        source: &dyn FrameSource,
    ) -> Result<(), BackendError> {
        let _ = (target, size, scale, source);
        Err(RenderError::NoTarget.into())
    }

    /// Renders `source` alone into a new texture of `size` at `scale` that stays on this GPU, its
    /// bottom corners cut by `radius` logical pixels, and returns once it is done: the last image
    /// of a closing window, with no CPU read. The texture is this renderer's own; only it draws
    /// it (see [`body_copy`]).
    fn render_window_texture(
        &mut self,
        size: PhysicalSize,
        scale: Scale,
        radius: u32,
        source: &dyn FrameSource,
    ) -> Result<GlesTexture, BackendError> {
        let _ = (size, scale, radius, source);
        Err(RenderError::NoTarget.into())
    }
}

/// How a closing window's last image is kept.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BodyCopy {
    /// Copied into a texture on the display GPU, never read back.
    Gpu,
    /// Read back to the CPU and drawn from a memory buffer on any renderer.
    ReadBack,
}

/// How to keep a closing window's last image on a seat with `devices` open display devices, of
/// which `lending` lend a renderer. A texture belongs to one GL context, so the GPU copy is for a
/// seat whose only device lends its renderer: with none (software) or a second GPU, whose
/// scanout would draw it from another context, the image is read back.
///
/// ```
/// use casement_backend::tty::{BodyCopy, body_copy};
///
/// assert_eq!(body_copy(1, 1), BodyCopy::Gpu);
/// assert_eq!(body_copy(1, 0), BodyCopy::ReadBack);
/// assert_eq!(body_copy(2, 2), BodyCopy::ReadBack);
/// ```
pub fn body_copy(devices: usize, lending: usize) -> BodyCopy {
    match (devices, lending) {
        (1, 1) => BodyCopy::Gpu,
        _ => BodyCopy::ReadBack,
    }
}

/// Whether a lender keeps watched outputs' frames on the GPU.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SceneSupport {
    /// It does.
    Drawn,
    /// It does not: the software shadow reads frames back.
    Absent,
}

/// What an output's frame looks like: the mode's size, the scale and the transform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Picture {
    /// The mode's size in physical pixels.
    pub size: PhysicalSize,
    /// The output's scale.
    pub scale: Scale,
    /// The output's transform.
    pub transform: Transform,
}

impl Picture {
    /// The picture an output shows with `config`.
    pub fn of(config: &OutputConfig) -> Picture {
        Picture {
            size: config.mode.size,
            scale: config.scale,
            transform: config.transform,
        }
    }

    /// Whether `area` has pixels and lies wholly inside the picture.
    pub fn holds(&self, area: PhysicalRect) -> bool {
        let right = i64::from(area.x) + i64::from(area.w);
        let bottom = i64::from(area.y) + i64::from(area.h);
        area.x >= 0
            && area.y >= 0
            && area.w > 0
            && area.h > 0
            && right <= i64::from(self.size.w)
            && bottom <= i64::from(self.size.h)
    }
}

/// Which renderer answers a dma-buf capture frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CaptureRoute {
    /// The display GPU's renderer.
    Gpu,
    /// The software renderer (linear buffers).
    Software,
}

/// Which renderer to try first for a buffer of `target` format: the GPU when it lists the format
/// with that modifier, else the software renderer.
pub fn capture_route(target: Format, gpu: &[Format]) -> CaptureRoute {
    match gpu.contains(&target) {
        true => CaptureRoute::Gpu,
        false => CaptureRoute::Software,
    }
}

/// What to do when the GPU could not draw into a buffer of `target` format: the software
/// renderer when it can bind that format and modifier, else nothing (the frame fails).
///
/// ```
/// use casement_backend::tty::{CaptureRoute, capture_fallback};
/// use smithay::backend::allocator::{Format, Fourcc, Modifier};
///
/// let linear = Format { code: Fourcc::Argb8888, modifier: Modifier::Linear };
/// let tiled = Format { code: Fourcc::Argb8888, modifier: Modifier::from(0x0100_0000_0000_0001_u64) };
/// assert_eq!(capture_fallback(linear, &[linear]), Some(CaptureRoute::Software));
/// assert_eq!(capture_fallback(tiled, &[linear]), None);
/// ```
pub fn capture_fallback(target: Format, software: &[Format]) -> Option<CaptureRoute> {
    software.contains(&target).then_some(CaptureRoute::Software)
}

/// `first` followed by the formats of `second` it does not have: the offer of both renderers.
pub fn merged_formats(first: Vec<Format>, second: Vec<Format>) -> Vec<Format> {
    let mut all = first;
    for format in second {
        if !all.contains(&format) {
            all.push(format);
        }
    }
    all
}

/// The `Argb8888` and `Xrgb8888` formats `renderer` binds a dma-buf as a render target with.
pub fn renderable_formats<R: Bind<Dmabuf>>(renderer: &R) -> Vec<Format> {
    Bind::<Dmabuf>::supported_formats(renderer)
        .map(|set| {
            set.iter()
                .filter(|format| matches!(format.code, Fourcc::Argb8888 | Fourcc::Xrgb8888))
                .copied()
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use smithay::backend::allocator::Modifier;

    fn format(code: Fourcc, modifier: Modifier) -> Format {
        Format { code, modifier }
    }

    fn tiled() -> Modifier {
        Modifier::from(0x0100_0000_0000_0001_u64)
    }

    #[test]
    fn a_picture_holds_a_region_only_when_it_is_inside() {
        let picture = Picture {
            size: PhysicalSize { w: 640, h: 480 },
            scale: Scale(120),
            transform: Transform::Normal,
        };
        let rect = |x, y, w, h| PhysicalRect { x, y, w, h };
        // name, the region, whether it is held
        let table = [
            ("whole", rect(0, 0, 640, 480), true),
            ("one pixel", rect(10, 20, 1, 1), true),
            ("the last pixel", rect(639, 479, 1, 1), true),
            ("one past the right edge", rect(640, 0, 1, 1), false),
            ("one past the bottom edge", rect(0, 480, 1, 1), false),
            ("sticks out", rect(600, 0, 41, 1), false),
            ("before the origin", rect(-1, 0, 2, 2), false),
            ("no area", rect(5, 5, 0, 1), false),
        ];
        for (name, area, held) in table {
            assert_eq!(picture.holds(area), held, "{name}");
        }
    }

    #[test]
    fn the_route_follows_what_the_gpu_lists() {
        let argb = Fourcc::Argb8888;
        let gpu = [format(argb, tiled()), format(argb, Modifier::Linear)];
        // name, the client's buffer, the route
        let table = [
            ("gpu modifier", format(argb, tiled()), CaptureRoute::Gpu),
            (
                "linear the gpu lists",
                format(argb, Modifier::Linear),
                CaptureRoute::Gpu,
            ),
            (
                "another format",
                format(Fourcc::Xrgb8888, Modifier::Linear),
                CaptureRoute::Software,
            ),
            (
                "unknown modifier",
                format(argb, Modifier::from(7_u64)),
                CaptureRoute::Software,
            ),
        ];
        for (name, target, route) in table {
            assert_eq!(capture_route(target, &gpu), route, "{name}");
        }
        assert_eq!(
            capture_route(format(argb, Modifier::Linear), &[]),
            CaptureRoute::Software,
            "no gpu lent: software"
        );
    }

    #[test]
    fn a_closing_image_stays_on_the_gpu_only_on_a_seat_with_one_lending_device() {
        // name, open devices, devices lending a renderer, the copy
        let table = [
            ("one gpu", 1, 1, BodyCopy::Gpu),
            ("one device, software", 1, 0, BodyCopy::ReadBack),
            ("no device", 0, 0, BodyCopy::ReadBack),
            ("a second gpu", 2, 2, BodyCopy::ReadBack),
            (
                "a second device that does not lend",
                2,
                1,
                BodyCopy::ReadBack,
            ),
        ];
        for (name, devices, lending, copy) in table {
            assert_eq!(body_copy(devices, lending), copy, "{name}");
        }
    }

    #[test]
    fn after_a_gpu_fault_only_a_format_the_software_binds_is_retried() {
        let argb = Fourcc::Argb8888;
        let software = [format(argb, Modifier::Linear)];
        assert_eq!(
            capture_fallback(format(argb, Modifier::Linear), &software),
            Some(CaptureRoute::Software)
        );
        assert_eq!(capture_fallback(format(argb, tiled()), &software), None);
        assert_eq!(capture_fallback(format(argb, Modifier::Linear), &[]), None);
    }

    #[test]
    fn the_offer_is_the_gpus_formats_then_the_softwares_new_ones() {
        let argb = Fourcc::Argb8888;
        let gpu = vec![format(argb, tiled()), format(argb, Modifier::Linear)];
        let software = vec![
            format(argb, Modifier::Linear),
            format(Fourcc::Xrgb8888, Modifier::Linear),
        ];
        assert_eq!(
            merged_formats(gpu.clone(), software),
            [
                format(argb, tiled()),
                format(argb, Modifier::Linear),
                format(Fourcc::Xrgb8888, Modifier::Linear)
            ],
            "linear once, the gpu's order first"
        );
        assert_eq!(merged_formats(Vec::new(), gpu.clone()), gpu);
    }
}
