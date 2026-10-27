//! The closed set of things casement draws. Each delta freeze adds variants (P3: shadow,
//! rounded clip (built), dim, genie, thumbnail, tile preview, chrome bitmap; P4: glow, agent cursor,
//! click ring, trust frame).

mod blur_v0;
mod casement_element;
mod client_round;
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
mod overview_icon;
mod popup;
mod reduced;
mod rounded_clip;
mod shared;
mod soft_edge;
mod solid;
mod strip;
mod tile_preview;
mod window;

pub use blur_v0::BlurV0Element;
pub use client_round::{ClientRound, ClientShadow, ClipAsk};
pub use clip_strip::ClipStripElement;
pub use closing::{ClosingFrame, closing_content, closing_elements};
pub use closing_body::{ClosingBody, Pixels};
pub use cursor::{CursorFace, MAX_CLIENT_CURSOR, cursor_elements, cursor_face, cursor_pixels};
pub use flight::{
    FlightDraw, at_rest, choose, drawn_content, rest_place, scale_ratio, scaled_content,
};
pub use frame::{
    FrameDraw, client_shadow_elements, frame_elements, label_element, shown_length, shown_outline,
};
pub(crate) use frame::{Raster, ShadowKey, ShadowSet, Tiles, WholeShadow};
#[cfg(test)]
pub(crate) use frame::{shadow_key, shadow_tiles};
pub use genie::{GenieParts, GenieWindow, SoftParts, genie_elements, genie_window, strip_plans};
pub use geometry::{geometry_size, margin_of, surface_origin, surface_origin_logical};
pub use layer::{layer_elements, layer_elements_faded};
pub use lock_cover::LockCoverElement;
pub use overview::{OverviewLook, dim_elements, lift_last, ring_elements};
pub use overview_icon::{IconPlace, OverviewIcon, icon_element, icon_place};
pub use popup::{PopupRide, popup_elements, popup_elements_riding, popup_surfaces};
pub use reduced::{BlurFinish, under_finish};
pub(crate) use rounded_clip::bake_corners;
pub use rounded_clip::{ClipShape, RoundedClipElement, Rounding};
pub use shared::SharedElement;
pub use soft_edge::{Part, Weight};
pub use solid::{Rgba8, solid_rect};
pub(crate) use strip::pixman_can_draw;
pub use strip::{Margins, StripElement, StripPlan};
pub use tile_preview::{TilePreview, tile_preview_elements};
pub(crate) use window::{alpha, flight_ratio};
pub use window::{window_content, window_drawn, window_elements, window_surfaces};

pub use casement_element::CasementElement;
