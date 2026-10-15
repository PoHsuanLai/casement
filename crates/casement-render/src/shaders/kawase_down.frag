#version 100

//_DEFINES_

// Dual Kawase, downsample (Bjorge, "Bandwidth-Efficient Rendering", SIGGRAPH 2015): the centre
// four times and the four diagonal taps once, over eight. `texel` is one source texel in texture
// coordinates; the drawn target is half the source's size, so each tap is a bilinear sample
// over a 2 by 2 block. Written from the paper.

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
    vec4 sum = tap(v_coords) * 4.0;
    sum += tap(v_coords + vec2(-d.x, -d.y));
    sum += tap(v_coords + vec2(d.x, -d.y));
    sum += tap(v_coords + vec2(-d.x, d.y));
    sum += tap(v_coords + vec2(d.x, d.y));
    vec4 color = sum / 8.0 * alpha;

#if defined(DEBUG_FLAGS)
    if (tint == 1.0)
        color = vec4(0.0, 0.2, 0.0, 0.2) + color * 0.8;
#endif

    gl_FragColor = color;
}
