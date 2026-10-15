//! The text of a server-side window frame: the title, laid out with parley (fontique's system
//! fonts), truncated with an ellipsis at its end, and rasterised by vello_cpu to premultiplied
//! RGBA8 at the output's native scale. The only crate that reaches the text stack (boundary row
//! B15); everything else in casement takes the pixels as data.
//!
//! The font comes from quire's `title_font` token (family, 13 px, weight 600), never from a number
//! written here; the ink from quire's `Ink` and `InkSoft` colours, the secondary one for a window
//! that is not the key window. Pure parts ([`ink`], [`place`], [`TitleKey`]) are table-tested; the
//! shaping needs fonts and is tested for its invariants.

mod cache;
mod error;
pub mod ink;
pub mod key;
pub mod pixels;
pub mod place;
mod raster;

pub use error::TitleError;
pub use ink::{Ink, ink};
pub use key::TitleKey;
pub use pixels::TitlePixels;
pub use place::Margins;
pub use raster::TitleRaster;
