//! The app's icon under a window in Mission Control: a raster made elsewhere (`casement-icons`
//! finds and decodes it), laid centred under the window, overlapping its bottom edge. It arrives
//! with the window's slot: from nothing, as small and as faint as the window is far from it, to
//! the layout's rectangle at rest, and leaves the same way.

use super::frame::{TitleDraw, element, to_buffer, whole};
use super::{CasementElement, Raster};
use crate::compositor::Compositor;
use crate::scene::OutputView;
use crate::texture_book::TextureBook;
use casement_core::WindowId;
use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize, buffer_size};

/// One window's icon in Mission Control.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverviewIcon {
    /// Where the layout puts the icon when the window is in its slot (global logical pixels).
    pub rest: LogicalRect,
    /// The window's slot (the same space).
    pub slot: LogicalRect,
    /// The icon's pixels, premultiplied, as big as `rest` at the output's native scale.
    pub draw: TitleDraw,
}

/// Where an icon is drawn this frame and how opaque.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IconPlace {
    /// Its rectangle, global logical pixels.
    pub at: LogicalRect,
    /// Its opacity, 0 to 1.
    pub alpha: f32,
}

/// How far a window drawn `drawn` wide has come from `full` wide (its size on the desktop) to
/// `rest` wide (its slot): 0 at the start, 1 at the slot.
fn arrival(full: u32, drawn: u32, rest: u32) -> f64 {
    match full.saturating_sub(rest) {
        0 => 1.0,
        span => (f64::from(full.saturating_sub(drawn)) / f64::from(span)).clamp(0.0, 1.0),
    }
}

/// Where `icon` goes while its window's content is drawn at `content` (`full` is the window's
/// size on the desktop): the layout's offset from the slot's bottom middle, scaled by how far
/// the window has arrived, and the icon grown and faded by the same. Nothing before it starts.
///
/// ```
/// use casement_core::WindowId;
/// use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize};
/// use casement_render::{OverviewIcon, TitleDraw, icon_place};
/// use std::sync::Arc;
///
/// let rect = |x, y, w, h| LogicalRect::new(LogicalPoint { x, y }, LogicalSize { w, h });
/// let icon = OverviewIcon {
///     rest: rect(168, 374, 64, 64),
///     slot: rect(100, 100, 200, 300),
///     draw: TitleDraw { id: 1, width: 64, height: 64, rgba: Arc::from(vec![0u8; 64 * 64 * 4]) },
/// };
/// let full = LogicalSize { w: 400, h: 600 };
/// let at_rest = icon_place(&icon, icon.slot, full).unwrap();
/// assert_eq!((at_rest.at, at_rest.alpha), (icon.rest, 1.0));
/// assert!(icon_place(&icon, rect(0, 0, 400, 600), full).is_none());
/// ```
pub fn icon_place(
    icon: &OverviewIcon,
    content: LogicalRect,
    full: LogicalSize,
) -> Option<IconPlace> {
    let k = arrival(full.w, content.size.w, icon.slot.size.w);
    if k <= 0.0 {
        return None;
    }
    let half = |a: i32, b: u32| f64::from(a) + f64::from(b) / 2.0;
    let off_x =
        half(icon.rest.origin.x, icon.rest.size.w) - half(icon.slot.origin.x, icon.slot.size.w);
    let off_y = half(icon.rest.origin.y, icon.rest.size.h)
        - (f64::from(icon.slot.origin.y) + f64::from(icon.slot.size.h));
    let edge = (f64::from(icon.rest.size.w) * k).round().max(1.0);
    let middle = (
        half(content.origin.x, content.size.w) + off_x * k,
        f64::from(content.origin.y) + f64::from(content.size.h) + off_y * k,
    );
    let corner = |m: f64| i32::try_from(((m - edge / 2.0).round()) as i64).unwrap_or(0);
    let side = u32::try_from(edge as i64).unwrap_or(1);
    Some(IconPlace {
        at: LogicalRect::new(
            LogicalPoint {
                x: corner(middle.0),
                y: corner(middle.1),
            },
            LogicalSize { w: side, h: side },
        ),
        alpha: k as f32,
    })
}

/// `window`'s `icon` drawn at `place`: its raster uploaded once (kept until Mission Control closes) and
/// scaled by the GPU to the rectangle. Nothing for a raster that is not the size `rest` needs at
/// the output's scale, or a place off this output.
pub fn icon_element<R: Compositor>(
    window: WindowId,
    icon: &OverviewIcon,
    place: IconPlace,
    view: &OutputView,
    book: &TextureBook,
    renderer: &mut R,
) -> Option<CasementElement<R>> {
    view.area.intersection(&place.at)?;
    let pixels = buffer_size(icon.rest.size, view.scale);
    let draw = &icon.draw;
    if (draw.width, draw.height) != (pixels.w, pixels.h) || draw.argb().is_none() {
        return None;
    }
    let buffer = book
        .cache()
        .icon((window, draw.id, draw.width, draw.height), || {
            to_buffer(&Raster {
                w: draw.width,
                h: draw.height,
                argb: draw.argb().unwrap_or_default(),
            })
        });
    element(
        renderer,
        view,
        place.at.origin,
        place.at.size,
        whole(pixels.w, pixels.h),
        place.alpha,
        &buffer,
    )
    .map(CasementElement::Memory)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn rect(x: i32, y: i32, w: u32, h: u32) -> LogicalRect {
        LogicalRect::new(LogicalPoint { x, y }, LogicalSize { w, h })
    }

    fn icon() -> OverviewIcon {
        OverviewIcon {
            rest: rect(168, 374, 64, 64),
            slot: rect(100, 100, 200, 300),
            draw: TitleDraw {
                id: 1,
                width: 64,
                height: 64,
                rgba: Arc::from(vec![0u8; 64 * 64 * 4]),
            },
        }
    }

    #[test]
    fn the_icon_arrives_with_its_window() {
        let full = LogicalSize { w: 400, h: 600 };
        // name, content width (height keeps the 2:3), want (the opacity, or none)
        let cases = [
            ("not started", 400, None),
            ("a quarter in", 350, Some(0.25)),
            ("half way", 300, Some(0.5)),
            ("at the slot", 200, Some(1.0)),
            ("past the slot", 150, Some(1.0)),
        ];
        for (name, width, want) in cases {
            let content = rect(0, 0, width, width * 3 / 2);
            let got = icon_place(&icon(), content, full).map(|place| place.alpha);
            assert_eq!(got, want, "{name}");
        }
    }

    #[test]
    fn the_icon_hangs_off_the_drawn_bottom_by_the_scaled_offset() {
        let full = LogicalSize { w: 400, h: 600 };
        let content = rect(50, 60, 300, 450);
        let got = icon_place(&icon(), content, full).expect("started").at;
        // k = 1/2: a 32 px icon; the rest middle is 6 below the slot's bottom, half of that here.
        assert_eq!(got.size, LogicalSize { w: 32, h: 32 });
        assert_eq!(got.origin.x + 16, 50 + 150);
        assert_eq!(got.origin.y + 16, 60 + 450 + 3);
    }

    #[test]
    fn a_window_that_starts_at_the_slot_size_has_its_icon_at_once() {
        let slot = icon().slot;
        let got = icon_place(&icon(), slot, slot.size).expect("arrived");
        assert_eq!((got.at, got.alpha), (icon().rest, 1.0));
    }
}
