//! The rounded clip on GLES: the surface's texture drawn through a texture shader that
//! multiplies the pixel by the shape's coverage (`ClipShape::coverage`, the same formula).
//!
//! The shader finds where it is in the element by undoing smithay's texture matrix: for an
//! upright texture, `v_coords = (position * ratio + src.loc) / texture size`, so
//! `position = v_coords * (texture size * ratio) - src.loc * ratio`, where `position` is in
//! physical pixels from the element's top-left and `ratio` is `dst.size / src.size`. A texture
//! that is turned, flipped or upside down, and a single-pixel buffer, take the renderer-neutral
//! draw in `bands.rs` instead. The compiled shader is kept in the EGL context's user data, one
//! per renderer.

use super::{ClipShape, RoundedClipElement};
use smithay::backend::renderer::element::Element;
use smithay::backend::renderer::element::surface::{
    WaylandSurfaceRenderElement, WaylandSurfaceTexture,
};
use smithay::backend::renderer::gles::{
    GlesError, GlesFrame, GlesRenderer, GlesTexProgram, GlesTexture, Uniform, UniformName,
    UniformType,
};
use smithay::backend::renderer::{FrameContext, Texture};
use smithay::utils::user_data::UserDataMap;
use smithay::utils::{Buffer as BufferCoords, Physical, Rectangle, Transform};
use std::cell::RefCell;

/// The texture shader: smithay's default one with the corner mask. `from` and `to` turn the
/// texture coordinate into the pixel position in the element (`position = v_coords * to + from`),
/// `shape` is the window rectangle (x, y, w, h) in those pixels and `radius` its bottom corners'.
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
uniform vec2 to;
uniform vec2 from;
uniform vec4 shape;
uniform float radius;
varying vec2 v_coords;

#if defined(DEBUG_FLAGS)
uniform float tint;
#endif

float coverage(vec2 p) {
    vec2 half_size = shape.zw * 0.5;
    vec2 centre = shape.xy + half_size;
    float r = p.y > centre.y ? radius : 0.0;
    vec2 q = abs(p - centre) - half_size + vec2(r);
    float edge = min(max(q.x, q.y), 0.0) + length(max(q, vec2(0.0))) - r;
    return clamp(0.5 - edge, 0.0, 1.0);
}

void main() {
    vec4 color = texture2D(tex, v_coords);

#if defined(NO_ALPHA)
    color = vec4(color.rgb, 1.0) * alpha;
#else
    color = color * alpha;
#endif

    color = color * coverage(v_coords * to + from);

#if defined(DEBUG_FLAGS)
    if (tint == 1.0)
        color = vec4(0.0, 0.2, 0.0, 0.2) + color * 0.8;
#endif

    gl_FragColor = color;
}
";

/// The compiled shader, kept in the EGL context's user data.
#[derive(Debug, Default)]
struct Compiled(RefCell<Option<GlesTexProgram>>);

/// Where the texture's pixels land in the space the shape is in: the source and destination
/// rectangles that turn a texture coordinate into a position in the shape's pixels. For a window
/// drawn as it is these are the draw's own; for a strip of a genie they are the window's at
/// rest, so the corners are cut where the window's are and the strip carries them along.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Mask {
    /// The shape, in the mask destination's space (output pixels).
    pub(crate) shape: ClipShape,
    /// The source rectangle of the unmoved element.
    pub(crate) src: Rectangle<f64, BufferCoords>,
    /// The destination rectangle of the unmoved element.
    pub(crate) dst: Rectangle<i32, Physical>,
}

/// Draws `clip`'s surface where `damage` (relative to `dst`) asks, masked to its shape.
pub(super) fn draw(
    frame: &mut GlesFrame<'_, '_>,
    clip: &RoundedClipElement<GlesRenderer>,
    src: Rectangle<f64, BufferCoords>,
    dst: Rectangle<i32, Physical>,
    damage: &[Rectangle<i32, Physical>],
    opaque_regions: &[Rectangle<i32, Physical>],
    cache: Option<&UserDataMap>,
) -> Result<(), GlesError> {
    let _ = cache;
    let mask = Mask {
        shape: clip.shape(),
        src,
        dst,
    };
    draw_masked(
        frame,
        clip.surface(),
        mask,
        (src, dst),
        (damage, opaque_regions),
        |frame| clip.draw_banded(frame, src, dst, damage, opaque_regions),
    )
}

/// The damage of a draw and the opaque regions, in physical pixels.
type Regions<'a> = (
    &'a [Rectangle<i32, Physical>],
    &'a [Rectangle<i32, Physical>],
);

/// Draws `surface` from `src` to `dst` (the rectangles of this draw) where `damage` asks,
/// through the corner mask of `mask`. A surface the shader cannot mask (no texture, or one that is
/// turned, flipped or empty) is drawn by `fallback` instead.
pub(crate) fn draw_masked(
    frame: &mut GlesFrame<'_, '_>,
    surface: &WaylandSurfaceRenderElement<GlesRenderer>,
    mask: Mask,
    (src, dst): (Rectangle<f64, BufferCoords>, Rectangle<i32, Physical>),
    (damage, opaque_regions): Regions<'_>,
    fallback: impl FnOnce(&mut GlesFrame<'_, '_>) -> Result<(), GlesError>,
) -> Result<(), GlesError> {
    let upright = surface.transform() == Transform::Normal;
    let WaylandSurfaceTexture::Texture(texture) = surface.texture() else {
        return fallback(frame);
    };
    if !upright || texture.is_y_inverted() || mask.src.size.w <= 0.0 || mask.src.size.h <= 0.0 {
        return fallback(frame);
    }
    let program = program(frame)?;
    let uniforms = uniforms(texture, mask);
    frame.render_texture_from_to(
        texture,
        src,
        dst,
        damage,
        opaque_regions,
        Transform::Normal,
        surface.alpha(),
        Some(&program),
        &uniforms,
    )
}

/// The shader's uniforms for `texture` placed as `mask` says.
fn uniforms(texture: &GlesTexture, mask: Mask) -> [Uniform<'static>; 4] {
    let (src, dst) = (mask.src, mask.dst);
    let size = texture.size();
    let ratio = (
        f64::from(dst.size.w) / src.size.w,
        f64::from(dst.size.h) / src.size.h,
    );
    let to = (f64::from(size.w) * ratio.0, f64::from(size.h) * ratio.1);
    let from = (-src.loc.x * ratio.0, -src.loc.y * ratio.1);
    let rect = mask.shape.rect();
    let shape = (
        (rect.x - dst.loc.x) as f32,
        (rect.y - dst.loc.y) as f32,
        rect.w as f32,
        rect.h as f32,
    );
    [
        Uniform::new("to", (to.0 as f32, to.1 as f32)),
        Uniform::new("from", (from.0 as f32, from.1 as f32)),
        Uniform::new("shape", shape),
        Uniform::new("radius", mask.shape.radius() as f32),
    ]
}

/// The shader, compiled on first use for this renderer's context.
fn program(frame: &mut GlesFrame<'_, '_>) -> Result<GlesTexProgram, GlesError> {
    let mut guard = frame.renderer();
    let renderer: &mut GlesRenderer = guard.as_mut();
    renderer
        .egl_context()
        .user_data()
        .insert_if_missing(Compiled::default);
    let known = renderer
        .egl_context()
        .user_data()
        .get::<Compiled>()
        .and_then(|compiled| compiled.0.borrow().clone());
    if let Some(program) = known {
        return Ok(program);
    }
    let program = renderer.compile_custom_texture_shader(
        SHADER,
        &[
            UniformName::new("to", UniformType::_2f),
            UniformName::new("from", UniformType::_2f),
            UniformName::new("shape", UniformType::_4f),
            UniformName::new("radius", UniformType::_1f),
        ],
    )?;
    if let Some(compiled) = renderer.egl_context().user_data().get::<Compiled>() {
        *compiled.0.borrow_mut() = Some(program.clone());
    }
    Ok(program)
}
