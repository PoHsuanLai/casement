//! The colours of the compositor's own scenes (Mission Control, the Split View backdrop), read
//! from quire's tokens so they follow the desktop's look. Pure; the loop turns them into the
//! renderer's colours. Nothing here is a copy of a number quire owns.

use crate::frame::colour;
use casement_core::window::FrameColour;
use ds_style::appearance::theme::Scheme;
use ds_style::tokens::colour::ColourToken;

/// The ring round the window the pointer is on in Mission Control: the accent's fill (the Mac's
/// blue until the person's accent is carried here).
///
/// ```
/// use casement_layout::{Scheme, overview_ring};
///
/// assert_eq!(overview_ring(Scheme::Light).a, 255);
/// ```
pub fn overview_ring(scheme: Scheme) -> FrameColour {
    colour(ColourToken::Accent.value(scheme))
}

/// The dimming over the picture in Mission Control: quire's scrim (black at about a fifth, the
/// same in both schemes).
///
/// ```
/// use casement_layout::{Scheme, overview_dim};
///
/// let dim = overview_dim(Scheme::Dark);
/// assert_eq!((dim.r, dim.g, dim.b), (0, 0, 0));
/// assert!(dim.a > 0 && dim.a < 128);
/// ```
pub fn overview_dim(scheme: Scheme) -> FrameColour {
    colour(ColourToken::Scrim.value(scheme))
}

/// What fills the output behind the halves of a Split View and in the gap between them: the
/// dark paper in either scheme (the Mac's backdrop stays dark in a light desktop).
///
/// ```
/// use casement_layout::split_backdrop;
///
/// assert_eq!(split_backdrop().a, 255);
/// ```
pub fn split_backdrop() -> FrameColour {
    colour(ColourToken::Paper.value(Scheme::Dark))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_colours_are_quires_tokens() {
        let paper = split_backdrop();
        assert_eq!((paper.r, paper.g, paper.b), (0x1E, 0x1E, 0x1E));
        // The scrim is .22 over nothing: 220 thousandths of 255.
        assert_eq!(overview_dim(Scheme::Light).a, 56);
        assert_eq!(overview_dim(Scheme::Light), overview_dim(Scheme::Dark));
        for scheme in [Scheme::Light, Scheme::Dark] {
            let ring = overview_ring(scheme);
            assert_eq!(ring.a, 255, "{scheme:?}");
            assert!(ring.b > ring.r, "{scheme:?}: the accent is blue");
        }
    }
}
