//! casement's renderer seam: what the compositor draws and how.
//!
//! [`Compositor`] is the renderer trait: smithay's [`Renderer`](smithay::backend::renderer::Renderer)
//! with the imports every element needs, plus whether effect shaders run. [`output_elements`]
//! turns one output's sampled [`casement_motion::OutputFrame`] into [`CasementElement`]s, and
//! [`render_output`] draws them through smithay's damage tracker. Feature `testing` adds
//! smithay's `DummyRenderer` and [`golden`], the golden-image comparison the test kit uses.
//!
//! Nothing here names a device backend (row B4): no DRM, udev, session or libinput.

mod compositor;
mod damage;
mod element;
mod gl_raw;
mod scene;
mod texture_book;

#[cfg(feature = "testing")]
pub mod golden;

pub use compositor::{Compositor, EffectSupport};
pub use damage::{RenderError, RenderSync, Rendered, render_output};
pub use element::{
    BlurV0Element, CasementElement, LockCoverElement, Rgba8, cursor_elements, layer_elements,
    popup_elements, popup_surfaces, solid_rect, window_elements,
};
pub use gl_raw::surfaceless_gles;
pub use scene::{ChromeLayer, CursorDraw, CursorShape, LayerDraw, OutputView, output_elements};
pub use texture_book::{SurfaceEntry, TextureBook};
