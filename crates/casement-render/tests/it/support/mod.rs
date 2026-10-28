//! Shared helpers for the renderer-only tests: offscreen targets on pixman and on llvmpipe
//! GLES, one render through `render_output`, the read-back, and the golden environment.

use casement_render::golden::{Golden, GoldenOutcome, GoldenTier, compare};
use casement_render::{CasementElement, Compositor, Rendered, render_output, surfaceless_gles};
use image::RgbaImage;
use smithay::backend::allocator::Fourcc;
use smithay::backend::renderer::damage::OutputDamageTracker;
use smithay::backend::renderer::gles::{GlesRenderbuffer, GlesRenderer};
use smithay::backend::renderer::pixman::PixmanRenderer;
use smithay::backend::renderer::{Bind, ExportMem, Offscreen};
use smithay::reexports::pixman::Image;
use smithay::utils::{Rectangle, Size, Transform};
use std::path::PathBuf;

/// A renderer with an offscreen target of one size: what a headless output renders into.
pub trait Offscreen1: Compositor + ExportMem {
    /// The target type.
    type Target;
    /// A target of `size` physical pixels in `Abgr8888` (bytes R, G, B, A).
    fn target(&mut self, size: (i32, i32)) -> Self::Target;
    /// Renders `elements` into `target` (`size` pixels) with `tracker` at buffer `age`, then
    /// reads it back.
    fn draw(
        &mut self,
        target: &mut Self::Target,
        tracker: &mut OutputDamageTracker,
        size: (i32, i32),
        elements: &[CasementElement<Self>],
        age: usize,
    ) -> (Rendered, Vec<u8>);
}

macro_rules! offscreen {
    ($renderer:ty, $target:ty) => {
        impl Offscreen1 for $renderer {
            type Target = $target;

            fn target(&mut self, size: (i32, i32)) -> $target {
                Offscreen::<$target>::create_buffer(self, Fourcc::Abgr8888, Size::from(size))
                    .expect("offscreen target")
            }

            fn draw(
                &mut self,
                target: &mut $target,
                tracker: &mut OutputDamageTracker,
                size: (i32, i32),
                elements: &[CasementElement<Self>],
                age: usize,
            ) -> (Rendered, Vec<u8>) {
                let size = Size::from(size);
                let mut framebuffer = self.bind(target).expect("bind");
                let rendered = render_output(self, &mut framebuffer, tracker, elements, age)
                    .expect("render_output");
                let mapping = self
                    .copy_framebuffer(&framebuffer, Rectangle::from_size(size), Fourcc::Abgr8888)
                    .expect("copy_framebuffer");
                let bytes = self.map_texture(&mapping).expect("map").to_vec();
                (rendered, bytes)
            }
        }
    };
}

offscreen!(PixmanRenderer, Image<'static, 'static>);
offscreen!(GlesRenderer, GlesRenderbuffer);

/// The key of a headless output named `name` (no EDID serial).
pub fn output_key(name: &str) -> casement_core::OutputKey {
    use casement_core::output::{EdidFacts, EdidSerial};
    let facts = EdidFacts {
        make: "casement".to_owned(),
        model: "headless".to_owned(),
        serial: EdidSerial::Unknown,
    };
    let name = casement_core::OutputName::parse(name).expect("a valid output name");
    casement_core::OutputKey::of(&facts, &name)
}

/// A pixman renderer.
pub fn pixman() -> PixmanRenderer {
    PixmanRenderer::new().expect("pixman renderer")
}

/// GLES on llvmpipe (the jail pins Mesa's EGL vendor).
pub fn gles() -> GlesRenderer {
    surfaceless_gles().expect("surfaceless GLES on llvmpipe")
}

/// A tracker for an output of `size` physical pixels at `scale`.
pub fn tracker(size: (i32, i32), scale: f64) -> OutputDamageTracker {
    OutputDamageTracker::new(size, scale, Transform::Normal)
}

/// Renders `elements` once from scratch on a fresh target and tracker.
pub fn render_once<R: Offscreen1>(
    renderer: &mut R,
    size: (i32, i32),
    scale: f64,
    elements: &[CasementElement<R>],
) -> Vec<u8> {
    let mut target = renderer.target(size);
    let mut tracker = tracker(size, scale);
    renderer
        .draw(&mut target, &mut tracker, size, elements, 0)
        .1
}

/// `bytes` (R, G, B, A rows) as an image.
pub fn image_of(size: (i32, i32), bytes: Vec<u8>) -> RgbaImage {
    let side = |n: i32| u32::try_from(n).expect("positive size");
    RgbaImage::from_raw(side(size.0), side(size.1), bytes).expect("a whole frame")
}

/// Images side by side, top-aligned (outputs placed next to each other).
pub fn beside(images: &[RgbaImage]) -> RgbaImage {
    let width = images.iter().map(RgbaImage::width).sum();
    let height = images.iter().map(RgbaImage::height).max().unwrap_or(0);
    let mut out = RgbaImage::new(width, height);
    let mut x = 0;
    for image in images {
        for (ix, iy, pixel) in image.enumerate_pixels() {
            out.put_pixel(x + ix, iy, *pixel);
        }
        x += image.width();
    }
    out
}

/// Whether goldens are blessed: `CASEMENT_BLESS=1` only, the rule of the test kit's `Env`
/// (this crate cannot depend on the test kit, which depends on it).
pub fn bless() -> bool {
    std::env::var("CASEMENT_BLESS").ok().as_deref() == Some("1")
}

/// Where differing frames go.
pub fn out_dir() -> PathBuf {
    std::env::var_os("CASEMENT_GOLDEN_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("casement-golden"))
}

/// Compares `actual` with `tests/golden/<name>.png` under `tier`; panics with the outcome when
/// it differs.
pub fn check(name: &str, tier: GoldenTier, actual: &RgbaImage) {
    let golden = Golden {
        scene: format!("tests/golden/{name}"),
        tier,
    };
    let outcome = compare(&golden, actual, &out_dir(), bless());
    assert!(
        matches!(outcome, GoldenOutcome::Match | GoldenOutcome::Blessed),
        "{name}: {outcome:?}"
    );
}
