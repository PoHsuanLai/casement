#version 100

//_DEFINES_

// Dual Kawase, upsample (Bjorge 2015): eight taps, the four on the axes once and the four
// diagonals twice, over twelve. `texel` is one source texel (the smaller level's) in texture
// coordinates. The last upsample is also the finish: saturation about Rec. 709 luma, a
// premultiplied tint mixed over, a little grain, then the element's alpha. The passes in between
// run it with saturation 1, no tint and no grain. Written from the paper.

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
uniform float offset;
uniform float saturation;
uniform vec4 wash;
uniform float grain;
varying vec2 v_coords;

#if defined(DEBUG_FLAGS)
uniform float tint;
#endif

vec4 tap(vec2 at) {
    vec2 lo = texel * 0.5;
    return texture2D(tex, clamp(at, lo, vec2(1.0) - lo));
}

void main() {
    vec2 d = texel * offset;
    vec2 h = d * 0.5;
    vec4 sum = tap(v_coords + vec2(-d.x, 0.0));
    sum += tap(v_coords + vec2(d.x, 0.0));
    sum += tap(v_coords + vec2(0.0, -d.y));
    sum += tap(v_coords + vec2(0.0, d.y));
    sum += tap(v_coords + vec2(-h.x, -h.y)) * 2.0;
    sum += tap(v_coords + vec2(h.x, -h.y)) * 2.0;
    sum += tap(v_coords + vec2(-h.x, h.y)) * 2.0;
    sum += tap(v_coords + vec2(h.x, h.y)) * 2.0;
    vec4 color = sum / 12.0;

    // Premultiplied colour, so the saturation (linear in rgb) needs no division.
    float luma = dot(color.rgb, vec3(0.2126, 0.7152, 0.0722));
    color.rgb = clamp(mix(vec3(luma), color.rgb, saturation), 0.0, color.a);
    color = wash + color * (1.0 - wash.a);
    // Grain only when asked, and from a hash with no sin(): the usual sin(dot(p, k)) hash takes
    // an argument past 65536 at full-HD coordinates, where llvmpipe's sin returns NaN, and NaN
    // times a grain of 0 is still NaN (the pixel went black wherever 12.9898 x + 78.233 y was
    // over 65536). Interleaved gradient noise (Jimenez 2014) keeps every argument small.
    if (grain > 0.0) {
        float n = fract(52.9829189 * fract(dot(gl_FragCoord.xy, vec2(0.06711056, 0.00583715)))) - 0.5;
        color.rgb = clamp(color.rgb + vec3(n * grain) * color.a, 0.0, color.a);
    }
    color = color * alpha;

#if defined(DEBUG_FLAGS)
    if (tint == 1.0)
        color = vec4(0.0, 0.2, 0.0, 0.2) + color * 0.8;
#endif

    gl_FragColor = color;
}
