//! The one `unsafe` module of the casement workspace (row B18, 21-FREEZE-AMENDMENTS A17).
//!
//! smithay marks two constructors `unsafe fn`: `EGLDisplay::new` and `GlesRenderer::new`. This
//! module owns exactly those two calls and nothing else; every other GL pass goes through
//! smithay's safe shader API. A new `unsafe` block here needs a reviewed interface ask
//! (ARCHITECTURE.md section 6).
#![allow(unsafe_code)]

use crate::damage::RenderError;
use smithay::backend::egl::native::EGLSurfacelessDisplay;
use smithay::backend::egl::{EGLContext, EGLDisplay};
use smithay::backend::renderer::gles::GlesRenderer;

/// A GLES renderer on the EGL surfaceless platform: no DRM node, no window system. In the jail
/// this is Mesa's llvmpipe (the jail pins the Mesa EGL vendor), the headless backend's
/// `SoftRenderer::Llvmpipe` and the tier-L goldens' renderer (SP2).
pub fn surfaceless_gles() -> Result<GlesRenderer, RenderError> {
    // SAFETY: `EGLDisplay::new` is unsafe because a display obtained elsewhere through
    // `eglGetPlatformDisplay` with the same parameters could be terminated behind smithay's
    // back. Nothing in casement creates an EGL display except this function, and casement links
    // no other EGL user, so smithay's instance tracking holds.
    let display = unsafe { EGLDisplay::new(EGLSurfacelessDisplay) }
        .map_err(|error| RenderError::Renderer(format!("EGL surfaceless display: {error}")))?;
    let context = EGLContext::new(&display)
        .map_err(|error| RenderError::Renderer(format!("EGL context: {error}")))?;
    // SAFETY: `GlesRenderer::new` is undefined behaviour if the context is current on another
    // thread. The context was created on this line's thread a statement ago and has not been
    // shared or made current anywhere; the renderer takes ownership of it.
    unsafe { GlesRenderer::new(context) }
        .map_err(|error| RenderError::Renderer(format!("GLES renderer: {error}")))
}
