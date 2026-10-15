//! Blur v0 on GLES: one downsample-and-blur of what lies under the region.
//!
//! [`capture`] runs when smithay says the framebuffer under the element changed: it blits the
//! element's rectangle, a quarter of the size on each side with linear filtering, into a
//! texture kept in the element's cache. [`draw`] stretches that texture back over the region
//! through a 5 by 5 binomial texture shader, so the result is a smooth blur of roughly eight
//! output pixels' radius. Every call goes through smithay's safe GLES API.

use super::BlurV0Element;
use smithay::backend::allocator::Fourcc;
use smithay::backend::renderer::gles::{
    GlesError, GlesFrame, GlesRenderer, GlesTexProgram, GlesTexture, Uniform, UniformName,
    UniformType,
};
use smithay::backend::renderer::{Bind, BlitFrame, Frame, FrameContext, Offscreen, TextureFilter};
use smithay::utils::user_data::UserDataMap;
use smithay::utils::{Buffer as BufferCoords, Physical, Rectangle, Size, Transform};
use std::cell::RefCell;

/// How much smaller the captured copy is on each side.
const DOWNSAMPLE: i32 = 4;

/// The texture shader: smithay's default texture shader with a 5 by 5 binomial filter
/// (weights 1 4 6 4 1 on each axis) one texel apart in the downsampled copy.
const SHADER: &str = r"#version 100

//_DEFINES_

#if defined(EXTERNAL)
#extension GL_OES_EGL_image_external : require
#endif

precision highp float;
#if defined(EXTERNAL)
uniform samplerExternalOES tex;
#else
uniform sampler2D tex;
#endif

uniform float alpha;
uniform vec2 texel;
varying vec2 v_coords;

#if defined(DEBUG_FLAGS)
uniform float tint;
#endif

float weight(int i) {
    if (i == 0) { return 6.0; }
    if (i == -1 || i == 1) { return 4.0; }
    return 1.0;
}

void main() {
    vec4 sum = vec4(0.0);
    for (int y = -2; y <= 2; y++) {
        for (int x = -2; x <= 2; x++) {
            vec2 at = v_coords + vec2(float(x), float(y)) * texel;
            sum += texture2D(tex, at) * (weight(x) * weight(y));
        }
    }
    vec4 color = sum / 256.0 * alpha;

#if defined(DEBUG_FLAGS)
    if (tint == 1.0)
        color = vec4(0.0, 0.2, 0.0, 0.2) + color * 0.8;
#endif

    gl_FragColor = color;
}
";

/// What one blur element keeps between frames, in its damage-tracker cache.
#[derive(Debug, Default)]
struct BlurCache {
    /// The downsampled copy and the output rectangle it was taken from.
    copy: Option<Captured>,
    /// The compiled shader.
    program: Option<GlesTexProgram>,
}

/// A downsampled copy of the framebuffer under an element.
#[derive(Debug, Clone)]
struct Captured {
    /// The copy.
    texture: GlesTexture,
    /// Its size.
    size: Size<i32, BufferCoords>,
    /// The output rectangle it shows (the element's rectangle cut to the output).
    from: Rectangle<i32, Physical>,
}

/// Copies the framebuffer under `dst` (the element on the output) into the cache, downsampled.
pub(crate) fn capture(
    frame: &mut GlesFrame<'_, '_>,
    dst: Rectangle<i32, Physical>,
    cache: &UserDataMap,
) -> Result<(), GlesError> {
    cache.insert_if_missing(|| RefCell::new(BlurCache::default()));
    let Some(cell) = cache.get::<RefCell<BlurCache>>() else {
        return Ok(());
    };
    let Some(from) = dst.intersection(Rectangle::from_size(frame.output_size())) else {
        cell.borrow_mut().copy = None;
        return Ok(());
    };
    let size = downsampled(from.size);
    let reusable = cell
        .borrow()
        .copy
        .as_ref()
        .filter(|copy| copy.size == size)
        .map(|copy| copy.texture.clone());
    let mut texture = match reusable {
        Some(texture) => texture,
        None => {
            let mut guard = frame.renderer();
            Offscreen::<GlesTexture>::create_buffer(guard.as_mut(), Fourcc::Abgr8888, size)?
        }
    };
    {
        let mut target = {
            let mut guard = frame.renderer();
            let renderer: &mut GlesRenderer = guard.as_mut();
            renderer.bind(&mut texture)?
        };
        let to = Rectangle::from_size((size.w, size.h).into());
        let copied = frame.blit_to(&mut target, from, to, TextureFilter::Linear)?;
        frame.wait(&copied)?;
    }
    cell.borrow_mut().copy = Some(Captured {
        texture,
        size,
        from,
    });
    Ok(())
}

/// Draws the blurred copy over `blur`'s region where `damage` (relative to `dst`) asks; the
/// stand-in when nothing was captured yet.
pub(crate) fn draw(
    frame: &mut GlesFrame<'_, '_>,
    blur: &BlurV0Element,
    dst: Rectangle<i32, Physical>,
    damage: &[Rectangle<i32, Physical>],
    cache: Option<&UserDataMap>,
) -> Result<(), GlesError> {
    let Some(cell) = cache.and_then(|cache| cache.get::<RefCell<BlurCache>>()) else {
        return blur.draw_stand_in(frame, dst, damage);
    };
    let Some(copy) = cell.borrow().copy.clone() else {
        return blur.draw_stand_in(frame, dst, damage);
    };
    let program = program(frame, cell)?;
    // The region's damage, moved from the element's origin to the copy's.
    let shift = dst.loc - copy.from.loc;
    let local: Vec<_> = blur
        .clip(damage)
        .into_iter()
        .filter_map(|rect| {
            Rectangle::new(rect.loc + shift, rect.size)
                .intersection(Rectangle::from_size(copy.from.size))
        })
        .collect();
    if local.is_empty() {
        return Ok(());
    }
    let texel = Uniform::new(
        "texel",
        (1.0 / size_f32(copy.size.w), 1.0 / size_f32(copy.size.h)),
    );
    frame.render_texture_from_to(
        &copy.texture,
        Rectangle::from_size(copy.size).to_f64(),
        copy.from,
        &local,
        &[],
        Transform::Normal,
        blur.alpha_of(),
        Some(&program),
        &[texel],
    )
}

/// The cached shader, compiled on first use.
fn program(
    frame: &mut GlesFrame<'_, '_>,
    cell: &RefCell<BlurCache>,
) -> Result<GlesTexProgram, GlesError> {
    if let Some(program) = cell.borrow().program.clone() {
        return Ok(program);
    }
    let compiled = {
        let mut guard = frame.renderer();
        let renderer: &mut GlesRenderer = guard.as_mut();
        renderer
            .compile_custom_texture_shader(SHADER, &[UniformName::new("texel", UniformType::_2f)])?
    };
    cell.borrow_mut().program = Some(compiled.clone());
    Ok(compiled)
}

/// `size` divided by [`DOWNSAMPLE`], rounded up, at least one pixel.
fn downsampled(size: Size<i32, Physical>) -> Size<i32, BufferCoords> {
    let side = |n: i32| (n + DOWNSAMPLE - 1).div_euclid(DOWNSAMPLE).max(1);
    Size::from((side(size.w), side(size.h)))
}

/// A texture side as a float for the shader's texel step.
fn size_f32(n: i32) -> f32 {
    f32::from(u16::try_from(n.max(1)).unwrap_or(u16::MAX))
}
