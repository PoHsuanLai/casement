//! The closed set of things casement draws. Each delta freeze adds variants (P3: shadow,
//! rounded clip, dim, genie, thumbnail, tile preview, chrome bitmap; P4: glow, agent cursor,
//! click ring, trust frame).

mod blur_v0;
mod cursor;
mod frame;
mod geometry;
mod layer;
mod lock_cover;
mod popup;
mod solid;
mod window;

pub use blur_v0::BlurV0Element;
pub use cursor::cursor_elements;
pub use frame::frame_elements;
pub use geometry::{geometry_size, surface_origin, surface_origin_logical};
pub use layer::layer_elements;
pub use lock_cover::LockCoverElement;
pub use popup::{popup_elements, popup_surfaces};
pub use solid::{Rgba8, solid_rect};
pub use window::{window_content, window_elements};

use smithay::backend::renderer::element::memory::MemoryRenderBufferRenderElement;
use smithay::backend::renderer::element::solid::SolidColorRenderElement;
use smithay::backend::renderer::element::surface::WaylandSurfaceRenderElement;
use smithay::backend::renderer::{ImportAll, ImportMem};

smithay::backend::renderer::element::render_elements! {
    /// Every element casement draws.
    pub CasementElement<R> where R: ImportAll + ImportMem;
    /// A client surface (window, layer, popup, lock surface, cursor surface).
    Surface = WaylandSurfaceRenderElement<R>,
    /// A solid rectangle.
    Solid = SolidColorRenderElement,
    /// A CPU raster (chrome, the cursor theme).
    Memory = MemoryRenderBufferRenderElement<R>,
    /// The P0 behind-surface blur (ext-background-effect).
    Blur = BlurV0Element,
    /// The cover over a locked output without a live lock surface.
    Cover = LockCoverElement,
}
