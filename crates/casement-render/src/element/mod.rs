//! The closed set of things casement draws. Each delta freeze adds variants (P3: shadow,
//! rounded clip (built), dim, genie, thumbnail, tile preview, chrome bitmap; P4: glow, agent cursor,
//! click ring, trust frame).

mod blur_v0;
mod clip_strip;
mod closing;
mod closing_body;
mod cursor;
mod flight;
mod frame;
mod genie;
mod geometry;
mod layer;
mod lock_cover;
mod overview;
mod popup;
mod rounded_clip;
mod shared;
mod solid;
mod strip;
mod window;

pub use blur_v0::BlurV0Element;
pub use clip_strip::ClipStripElement;
pub use closing::{ClosingFrame, closing_content, closing_elements};
pub use closing_body::ClosingBody;
pub use cursor::{CursorFace, cursor_elements, cursor_face, cursor_pixels};
pub use flight::{
    FlightDraw, at_rest, choose, drawn_content, rest_place, scale_ratio, scaled_content,
};
pub use frame::{FrameDraw, TitleDraw, frame_elements, label_element, shown_length, shown_outline};
pub(crate) use frame::{Raster, ShadowKey, ShadowSet, Tiles, WholeShadow};
#[cfg(test)]
pub(crate) use frame::{shadow_key, shadow_tiles};
pub use genie::{GenieParts, GenieWindow, genie_elements, genie_window, strip_plans};
pub use geometry::{geometry_size, surface_origin, surface_origin_logical};
pub use layer::layer_elements;
pub use lock_cover::LockCoverElement;
pub use overview::{OverviewLook, dim_elements, ring_elements};
pub use popup::{PopupRide, popup_elements, popup_elements_riding, popup_surfaces};
pub use rounded_clip::{ClipShape, RoundedClipElement};
pub use shared::SharedElement;
pub use solid::{Rgba8, solid_rect};
pub use strip::{Margins, StripElement, StripPlan};
pub(crate) use window::{alpha, flight_ratio};
pub use window::{window_content, window_drawn, window_elements, window_surfaces};

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
    /// A framed window's surface clipped at its bottom corners (GLES; unclipped elsewhere).
    Clip = RoundedClipElement<R>,
    /// One strip of a genie's client surface.
    SurfaceStrip = StripElement<SharedElement<WaylandSurfaceRenderElement<R>>>,
    /// One strip of a genie's frame (titlebar and title).
    MemoryStrip = StripElement<SharedElement<MemoryRenderBufferRenderElement<R>>>,
    /// One strip of a genie's client surface that the window's rounded clip reaches.
    ClipStrip = ClipStripElement<R>,
}
