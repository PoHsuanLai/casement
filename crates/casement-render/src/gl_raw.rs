//! The one `unsafe` module of the casement workspace (row B18, 21-FREEZE-AMENDMENTS A17).
//!
//! smithay marks two constructors `unsafe fn`: `EGLDisplay::new` and `GlesRenderer::new`. This
//! module owns exactly those two calls, plus the two slices made from the raw pointer smithay
//! lends for a client's shm pool (P1-C1's capture writes; wave-2026-10-15/capture3's read of a
//! client-drawn cursor); every other GL pass goes through
//! smithay's safe shader API. A new `unsafe` block here needs a reviewed interface ask
//! (ARCHITECTURE.md section 6).
#![allow(unsafe_code)]

use crate::damage::RenderError;
use smithay::backend::egl::native::{EGLNativeDisplay, EGLSurfacelessDisplay};
use smithay::backend::egl::{EGLContext, EGLDisplay};
use smithay::backend::renderer::gles::GlesRenderer;
use smithay::reexports::wayland_server::protocol::wl_buffer::WlBuffer;
use smithay::wayland::shm::{
    BufferAccessError, BufferData, with_buffer_contents, with_buffer_contents_mut,
};

/// A GLES renderer on the EGL surfaceless platform: no DRM node, no window system. In the jail
/// this is Mesa's llvmpipe (the jail pins the Mesa EGL vendor), the headless backend's
/// `SoftRenderer::Llvmpipe` and the tier-L goldens' renderer (SP2).
pub fn surfaceless_gles() -> Result<GlesRenderer, RenderError> {
    gles_on(EGLSurfacelessDisplay)
}

/// A GLES renderer on the EGL display of `native` (a GBM device of a KMS node: the tty
/// backend's renderer for the outputs that device scans out, P1-T). The same two constructors
/// as [`surfaceless_gles`], over any native display, so this module still owns exactly two
/// `unsafe` calls.
pub fn gles_on<N: EGLNativeDisplay + 'static>(native: N) -> Result<GlesRenderer, RenderError> {
    // SAFETY: `EGLDisplay::new` is unsafe because a display obtained elsewhere through
    // `eglGetPlatformDisplay` with the same parameters could be terminated behind smithay's
    // back. Nothing in casement creates an EGL display except this function, and casement links
    // no other EGL user, so smithay's instance tracking holds.
    let display = unsafe { EGLDisplay::new(native) }
        .map_err(|error| RenderError::Renderer(format!("EGL display: {error}")))?;
    let context = EGLContext::new(&display)
        .map_err(|error| RenderError::Renderer(format!("EGL context: {error}")))?;
    // SAFETY: `GlesRenderer::new` is undefined behaviour if the context is current on another
    // thread. The context was created on this line's thread a statement ago and has not been
    // shared or made current anywhere; the renderer takes ownership of it.
    unsafe { GlesRenderer::new(context) }
        .map_err(|error| RenderError::Renderer(format!("GLES renderer: {error}")))
}

/// Runs `write` over the bytes of `buffer`'s shm pool, with the buffer's layout. smithay lends the
/// pool as a pointer and a length for the length of the call; this is where they become a slice.
pub(crate) fn with_shm_pool_mut<T>(
    buffer: &WlBuffer,
    write: impl FnOnce(&mut [u8], BufferData) -> T,
) -> Result<T, BufferAccessError> {
    with_buffer_contents_mut(buffer, |pointer, length, data| {
        // SAFETY: smithay lends `length` bytes at `pointer` for the length of this call and
        // nothing else in this process touches the pool meanwhile; the client may map it too,
        // which is the nature of shared memory (smithay's own docs say the same).
        let pool = unsafe { std::slice::from_raw_parts_mut(pointer, length) };
        write(pool, data)
    })
}

/// Runs `read` over the bytes of `buffer`'s shm pool, with the buffer's layout: the read-only twin
/// of [`with_shm_pool_mut`], for a cursor a client drew itself.
pub(crate) fn with_shm_pool<T>(
    buffer: &WlBuffer,
    read: impl FnOnce(&[u8], BufferData) -> T,
) -> Result<T, BufferAccessError> {
    with_buffer_contents(buffer, |pointer, length, data| {
        // SAFETY: smithay lends `length` bytes at `pointer` for the length of this call and the
        // compositor does not write the pool meanwhile; the client may write it, which is the
        // nature of shared memory (smithay's own docs say the same), and a torn read of a
        // cursor's pixels is a glitch, not a fault.
        let pool = unsafe { std::slice::from_raw_parts(pointer, length) };
        read(pool, data)
    })
}
