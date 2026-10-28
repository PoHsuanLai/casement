//! The colour pass on GLES: a texture drawn through a shader that does what
//! [`ColorTransform::apply`] does on the CPU (decode, matrix, sRGB encode). The compiled shader is
//! kept in the EGL context's user data, one per renderer. Only an output whose gamut is not sRGB
//! is ever drawn through it.

use super::color::{ColorTransform, selector};
use smithay::backend::renderer::gles::{
    GlesError, GlesFrame, GlesRenderer, GlesTexProgram, GlesTexture, Uniform, UniformName,
    UniformType, UniformValue,
};
use smithay::backend::renderer::{FrameContext, Texture};
use smithay::utils::{Buffer as BufferCoords, Physical, Rectangle, Transform};
use std::cell::RefCell;

/// The texture shader: smithay's default one with the colour conversion. `colour_matrix` takes linear
/// source RGB to linear panel RGB; `source_curve` picks the source curve (0 sRGB, 1 gamma 2.2, 2 linear).
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
uniform mat3 colour_matrix;
uniform float source_curve;
varying vec2 v_coords;

#if defined(DEBUG_FLAGS)
uniform float tint;
#endif

vec3 decode(vec3 v) {
    if (source_curve > 1.5) {
        return v;
    }
    if (source_curve > 0.5) {
        return pow(max(v, vec3(0.0)), vec3(2.2));
    }
    vec3 low = v / 12.92;
    vec3 high = pow((v + vec3(0.055)) / 1.055, vec3(2.4));
    return mix(high, low, step(v, vec3(0.04045)));
}

vec3 encode(vec3 l) {
    l = clamp(l, 0.0, 1.0);
    vec3 low = l * 12.92;
    vec3 high = 1.055 * pow(l, vec3(1.0 / 2.4)) - vec3(0.055);
    return mix(high, low, step(l, vec3(0.0031308)));
}

void main() {
    vec4 color = texture2D(tex, v_coords);

#if defined(NO_ALPHA)
    color = vec4(color.rgb, 1.0);
#endif

    // Textures hold premultiplied colour: convert the straight colour, premultiply again.
    vec3 straight = color.a > 0.0 ? min(color.rgb / color.a, vec3(1.0)) : color.rgb;
    vec3 converted = encode(colour_matrix * decode(straight));
    color = vec4(converted * color.a, color.a) * alpha;

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

/// Draws the whole of `texture` at its own size, one texel to one pixel, converted by
/// `transform`: the output's picture made ready for a wide-gamut panel.
pub fn draw_converted(
    frame: &mut GlesFrame<'_, '_>,
    texture: &GlesTexture,
    transform: &ColorTransform,
) -> Result<(), GlesError> {
    let size = texture.size();
    let dst = Rectangle::<i32, Physical>::from_size((size.w, size.h).into());
    let src =
        Rectangle::<f64, BufferCoords>::from_size((f64::from(size.w), f64::from(size.h)).into());
    let program = program(frame)?;
    let uniforms = uniforms(transform);
    frame.render_texture_from_to(
        texture,
        src,
        dst,
        &[dst],
        &[],
        Transform::Normal,
        1.0,
        Some(&program),
        &uniforms,
    )
}

/// The shader's uniforms for `transform`.
fn uniforms(transform: &ColorTransform) -> [Uniform<'static>; 2] {
    [
        Uniform::new(
            "colour_matrix",
            UniformValue::Matrix3x3 {
                matrices: vec![transform.matrix.column_major()],
                transpose: false,
            },
        ),
        Uniform::new("source_curve", selector(transform.source)),
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
            UniformName::new("colour_matrix", UniformType::Matrix3x3),
            UniformName::new("source_curve", UniformType::_1f),
        ],
    )?;
    if let Some(compiled) = renderer.egl_context().user_data().get::<Compiled>() {
        *compiled.0.borrow_mut() = Some(program.clone());
    }
    Ok(program)
}
