//! Sheets (P3-SH): a modal dialog of a window hangs from the bottom of that window's title bar,
//! centred on it, slides down out of the title bar when it opens, moves with its window, and
//! blocks the window's input until it closes (macOS Sonoma/Sequoia).
//!
//! A window is a sheet when it is modal for its parent (`xdg_dialog_v1.set_modal` on a
//! toplevel with a parent of the same app, or a `quire_window` hint that says the same): the
//! model has made that `Modality::WindowModal` and a parent window. What is here is the
//! geometry; layout puts the sheet where [`attach_rect`] says each time it arranges, so the
//! sheet follows its window with no bookkeeping, and the focus machine's sheet hook (A39)
//! already swallows presses on the window behind it.

use super::Window;
use crate::geometry::{LogicalPoint, LogicalRect, LogicalSize};
use crate::ids::WindowId;
use casement_protocols::vocab::Modality;

/// The window `window` is a sheet of, when it is one.
pub fn sheet_parent(window: &Window) -> Option<WindowId> {
    match (window.parent, window.modality) {
        (Some(parent), Modality::WindowModal) => Some(parent),
        _ => None,
    }
}

/// Where a sheet of `size` sits on a window whose content rectangle is `parent` (the title
/// bar is the strip just above it, so the sheet's top edge is the content's): centred
/// horizontally, its size held to the window's. A sheet never reaches past its window.
pub fn attach_rect(parent: LogicalRect, size: LogicalSize) -> LogicalRect {
    let size = LogicalSize {
        w: size.w.min(parent.size.w),
        h: size.h.min(parent.size.h),
    };
    let spare = i64::from(parent.size.w - size.w) / 2;
    let x = i64::from(parent.origin.x) + spare;
    LogicalRect::new(
        LogicalPoint {
            x: i32::try_from(x).unwrap_or(parent.origin.x),
            y: parent.origin.y,
        },
        size,
    )
}

/// Where the opening slide starts: the sheet raised by its own height, wholly behind the title
/// bar. The slide ends at `rect`; the renderer clips the sheet to [`slide_clip`] while it
/// moves, so it comes out of the title bar rather than over it.
pub fn slide_start(rect: LogicalRect) -> LogicalRect {
    let up = i64::from(rect.origin.y) - i64::from(rect.size.h);
    LogicalRect::new(
        LogicalPoint {
            x: rect.origin.x,
            y: i32::try_from(up).unwrap_or(i32::MIN),
        },
        rect.size,
    )
}

/// The rectangle a sliding sheet is visible in: its window's content rectangle. Closing
/// reverses the slide (the sheet goes back up to [`slide_start`]).
pub fn slide_clip(parent: LogicalRect) -> LogicalRect {
    parent
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: i32, y: i32, w: u32, h: u32) -> LogicalRect {
        LogicalRect::new(LogicalPoint { x, y }, LogicalSize { w, h })
    }

    fn size(w: u32, h: u32) -> LogicalSize {
        LogicalSize { w, h }
    }

    #[test]
    fn a_sheet_hangs_centred_from_the_top_of_the_content() {
        let parent = rect(100, 80, 800, 600);
        let cases = [
            ("centred", size(400, 200), rect(300, 80, 400, 200)),
            ("odd spare", size(401, 200), rect(299, 80, 401, 200)),
            ("as wide", size(800, 200), rect(100, 80, 800, 200)),
            (
                "wider than its window",
                size(900, 700),
                rect(100, 80, 800, 600),
            ),
        ];
        for (name, asked, want) in cases {
            assert_eq!(attach_rect(parent, asked), want, "{name}");
        }
    }

    #[test]
    fn a_moved_window_takes_its_sheet_along() {
        let asked = size(400, 200);
        let before = attach_rect(rect(100, 80, 800, 600), asked);
        let after = attach_rect(rect(160, 130, 800, 600), asked);
        assert_eq!(
            (
                after.origin.x - before.origin.x,
                after.origin.y - before.origin.y
            ),
            (60, 50)
        );
    }

    #[test]
    fn the_slide_starts_wholly_above_the_content() {
        let at = rect(300, 80, 400, 200);
        assert_eq!(slide_start(at), rect(300, -120, 400, 200));
        assert_eq!(slide_clip(rect(100, 80, 800, 600)), rect(100, 80, 800, 600));
    }
}
