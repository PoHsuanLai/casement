//! casement's renderer seam: what the compositor draws and how.
//!
//! [`Compositor`] is the renderer trait: smithay's [`Renderer`](smithay::backend::renderer::Renderer)
//! with the imports every element needs, plus whether effect shaders run. [`output_elements`]
//! turns one output's sampled [`casement_motion::OutputFrame`] into [`CasementElement`]s, and
//! [`render_output`] draws them through smithay's damage tracker. Feature `testing` adds
//! smithay's `DummyRenderer` and [`golden`], the golden-image comparison the test kit uses.
//!
//! Nothing here names a device backend (row B4): no DRM, udev, session or libinput.
//!
//! ```
//! use casement_core::WindowId;
//! use casement_render::TextureBook;
//!
//! let book = TextureBook::empty();
//! assert!(book.window(&WindowId(1)).is_none());
//! assert!(book.cursor().is_none());
//! ```

mod compositor;
mod damage;
mod draw_cache;
mod element;
mod gl_raw;
mod scene;
mod texture_book;

pub mod pass;

#[cfg(feature = "testing")]
pub mod golden;

pub use compositor::{Compositor, EffectSupport};
pub use damage::{RenderError, RenderSync, Rendered, render_output};
pub use element::{
    BlurV0Element, CasementElement, ClipShape, CursorFace, FlightDraw, FrameDraw, LockCoverElement,
    Rgba8, RoundedClipElement, SharedElement, StripElement, StripPlan, TitleDraw, at_rest, choose,
    cursor_elements, cursor_face, cursor_pixels, drawn_content, frame_elements, genie_elements,
    geometry_size, layer_elements, popup_elements, popup_surfaces, scale_ratio, solid_rect,
    strip_plans, surface_origin, surface_origin_logical, window_content, window_drawn,
    window_elements, window_surfaces,
};
pub use gl_raw::{gles_on, surfaceless_gles};
pub use scene::{ChromeLayer, CursorDraw, CursorShape, LayerDraw, OutputView, output_elements};
pub use texture_book::{SurfaceEntry, TextureBook};
