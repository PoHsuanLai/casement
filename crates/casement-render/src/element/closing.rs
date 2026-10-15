//! The frame of a window that has closed. A window that closes leaves the model at once, so
//! nothing says how it looked; the loop keeps a snapshot of its frame's draw data while the
//! window is alive ([`ClosingFrame`]) and hands it back when the window is gone but motion
//! still has it fading and shrinking. The frame (titlebar, lights, title, shadow) is then
//! drawn round the sampled rectangle at the sampled opacity, so it goes with the window as
//! one thing. When the loop also kept the window's last image ([`ClosingBody`]) the body is drawn
//! inside the frame, shrinking and fading with it; a window kept without one is not drawn.

use super::flight::{FlightDraw, scaled_content};
use super::{CasementElement, ClosingBody, FrameDraw, TitleDraw, frame_elements};
use crate::compositor::Compositor;
use crate::scene::OutputView;
use crate::texture_book::TextureBook;
use casement_core::geometry::{LogicalRect, LogicalSize, SubPoint, SubSize};
use casement_core::window::FrameStyle;
use casement_motion::PlacedWindow;

/// What the loop remembers of a window's frame, to draw it once the window has closed.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ClosingFrame {
    /// The frame's look and metrics as they were.
    pub style: FrameStyle,
    /// The window's last image, drawn inside the frame; none draws an empty body.
    pub body: Option<ClosingBody>,
}

impl ClosingFrame {
    /// A snapshot of the frame `style` makes, without a body.
    pub fn new(style: FrameStyle) -> ClosingFrame {
        ClosingFrame { style, body: None }
    }

    /// The same frame with the window's last image inside it.
    pub fn with_body(self, body: ClosingBody) -> ClosingFrame {
        ClosingFrame {
            body: Some(body),
            ..self
        }
    }
}

/// Where the content of a closing window is: its sampled place, at its sampled size (the
/// window shrinks a little as it fades).
///
/// ```
/// use casement_core::geometry::{SubPoint, SubPx, SubSize};
/// use casement_render::closing_content;
///
/// let at = SubPoint { x: SubPx(12_000), y: SubPx(24_000) };
/// let size = SubSize { w: SubPx(96_000), h: SubPx(72_000) };
/// let content = closing_content(at, size);
/// assert_eq!((content.origin.x, content.origin.y), (100, 200));
/// assert_eq!((content.size.w, content.size.h), (800, 600));
/// ```
pub fn closing_content(at: SubPoint, size: SubSize) -> LogicalRect {
    let origin = casement_core::geometry::LogicalPoint {
        x: at.x.round_logical(),
        y: at.y.round_logical(),
    };
    let none = LogicalRect::new(origin, LogicalSize { w: 0, h: 0 });
    scaled_content(FlightDraw::Scaled, none, size)
}

/// The frame of the closing window `placed`, from its snapshot, drawn at the window's
/// sampled opacity. Front first (titlebar, lights, title), then the body, then the shadow behind.
pub fn closing_elements<R: Compositor>(
    placed: &PlacedWindow,
    closing: &ClosingFrame,
    title: Option<&TitleDraw>,
    view: &OutputView,
    book: &TextureBook,
    renderer: &mut R,
) -> Vec<CasementElement<R>> {
    let content = closing_content(placed.at, placed.size);
    let draw = FrameDraw {
        style: &closing.style,
        title,
        content,
        opacity: placed.opacity,
    };
    let (front, back) = frame_elements(placed.window, draw, view, book, renderer);
    let body = closing
        .body
        .as_ref()
        .and_then(|body| body.element(content, placed.opacity, view, renderer));
    front.into_iter().chain(body).chain(back).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use casement_core::geometry::SubPx;

    #[test]
    fn a_closing_window_is_drawn_where_and_as_big_as_it_is_sampled() {
        // name, at (sub-pixels), size (sub-pixels), (x, y, w, h)
        let cases = [
            (
                "whole",
                (12_000, 24_000),
                (96_000, 72_000),
                (100, 200, 800, 600),
            ),
            (
                "tucked",
                (12_600, 24_600),
                (91_200, 68_400),
                (105, 205, 760, 570),
            ),
            ("nothing left", (0, 0), (-5, 0), (0, 0, 0, 0)),
            (
                "off the left",
                (-12_000, 0),
                (6_000, 6_000),
                (-100, 0, 50, 50),
            ),
        ];
        for (name, at, size, (x, y, w, h)) in cases {
            let content = closing_content(
                SubPoint {
                    x: SubPx(at.0),
                    y: SubPx(at.1),
                },
                SubSize {
                    w: SubPx(size.0),
                    h: SubPx(size.1),
                },
            );
            assert_eq!(
                (
                    content.origin.x,
                    content.origin.y,
                    content.size.w,
                    content.size.h
                ),
                (x, y, w, h),
                "{name}"
            );
        }
    }
}
