//! Picture in picture (P3-SH): a small video window that floats above every other window on
//! every Space, snaps to the nearest corner of the screen when the person lets go of it, and
//! keeps its aspect ratio when resized. Pure functions on rectangles; `model/pip.rs` carries
//! them into the model.
//!
//! A window is picture in picture when its client said so (`quire_window_v1.set_role(pip)`).
//! Browsers do not know that protocol, so a window that declared nothing and whose title and
//! app id are a known browser's picture-in-picture window is treated the same way
//! ([`PipSource::Guessed`]); a client that later sets a role of its own overrides the guess.

use super::{Window, WindowState};
use crate::app::AppKey;
use crate::geometry::{LogicalPoint, LogicalRect, LogicalSize};
use casement_protocols::vocab::WindowRole;

/// The gap between a snapped window and the screen's edge, in logical pixels.
pub const PIP_MARGIN: u32 = 16;

/// Why a window counts as picture in picture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipSource {
    /// Its client asked (`set_role(pip)`).
    Declared,
    /// Its title and app id are a browser's picture-in-picture window (a guess).
    Guessed,
}

/// The most area a window may have and still float above the rest, in logical pixels squared
/// (about 1140 x 560): a picture in picture window is a corner thumbnail, and one that fills
/// the screen is not one (CASC-11).
pub const PIP_MAX_AREA: u64 = 640_000;

/// The browsers whose picture-in-picture windows are recognised, by the start of the app id
/// (case-insensitive).
const BROWSER_IDS: [&str; 8] = [
    "firefox",
    "org.mozilla.firefox",
    "chromium",
    "google-chrome",
    "brave",
    "microsoft-edge",
    "vivaldi",
    "org.chromium",
];

/// The titles browsers give those windows (case-insensitive, spaces or hyphens alike).
const BROWSER_TITLES: [&str; 2] = ["picture-in-picture", "picture in picture"];

/// Whether `window` is picture in picture, and why.
///
/// ```
/// use casement_core::window::{PipSource, pip_source_of};
///
/// assert_eq!(pip_source_of(casement_protocols::vocab::WindowRole::Pip, "", ""), Some(PipSource::Declared));
/// assert_eq!(pip_source_of(casement_protocols::vocab::WindowRole::Document, "Mail", "org.quire.Mail"), None);
/// ```
pub fn pip_source_of(role: WindowRole, title: &str, app_id: &str) -> Option<PipSource> {
    match role {
        WindowRole::Pip => Some(PipSource::Declared),
        WindowRole::Document => browser_pip(title, app_id).then_some(PipSource::Guessed),
        WindowRole::Dialog | WindowRole::Utility | WindowRole::Shell => None,
    }
}

fn browser_pip(title: &str, app_id: &str) -> bool {
    let (title, app_id) = (title.to_ascii_lowercase(), app_id.to_ascii_lowercase());
    BROWSER_TITLES.contains(&title.as_str()) && BROWSER_IDS.iter().any(|id| app_id.starts_with(id))
}

/// Whether `window` floats above the rest. It must be picture in picture: declared by its
/// client, or guessed from a browser's title and app id, which counts only when the app is
/// proven (a title and an app id are text any client can write). And it must be small enough
/// ([`PIP_MAX_AREA`]) and loose (floating, not maximised or full screen).
pub fn floats_above(window: &Window) -> bool {
    let source = pip_source_of(
        window.role,
        window.title.0.as_str(),
        window.claimed_id.as_str(),
    );
    let believed = match (source, &window.app) {
        (Some(PipSource::Declared), _) | (Some(PipSource::Guessed), AppKey::Proven(_)) => true,
        (Some(PipSource::Guessed), AppKey::Claimed(_)) | (None, _) => false,
    };
    let size = match &window.state {
        WindowState::New(size) => Some(*size),
        WindowState::Floating(rect) => Some(rect.size),
        _ => None,
    };
    let small = size.is_some_and(|size| u64::from(size.w) * u64::from(size.h) <= PIP_MAX_AREA);
    believed && small
}

/// `rect` moved to the corner of `area` it is nearest to, [`PIP_MARGIN`] in from both edges.
/// The nearest corner is the one whose quadrant holds the rectangle's centre. A rectangle too
/// big for the margin sits flush with the corner instead.
pub fn snap_corner(rect: LogicalRect, area: LogicalRect) -> LogicalRect {
    let centre = rect.centre();
    let middle = area.centre();
    let at = |near_start: bool, start: i32, len: u32, extent: u32| -> i32 {
        let margin = i64::from(PIP_MARGIN.min(len.saturating_sub(extent) / 2));
        let value = match near_start {
            true => i64::from(start) + margin,
            false => i64::from(start) + i64::from(len) - i64::from(extent) - margin,
        };
        i32::try_from(value).unwrap_or(start)
    };
    LogicalRect::new(
        LogicalPoint {
            x: at(centre.x < middle.x, area.origin.x, area.size.w, rect.size.w),
            y: at(centre.y < middle.y, area.origin.y, area.size.h, rect.size.h),
        },
        rect.size,
    )
}

/// `wanted`, a resize of `held`, brought back to `held`'s aspect ratio. The side that changed
/// more (relatively) leads; the other follows. The edge that did not move stays: a window
/// dragged by its right edge keeps its left, one dragged by its bottom keeps its top, one
/// dragged by a corner keeps the opposite corner. A move (same size) is returned as it is, and
/// so is a resize of a window with no extent to take a ratio from.
pub fn fit_aspect(held: LogicalRect, wanted: LogicalRect) -> LogicalRect {
    if held.size == wanted.size || held.size.w == 0 || held.size.h == 0 {
        return wanted;
    }
    let (hw, hh) = (i64::from(held.size.w), i64::from(held.size.h));
    let (ww, wh) = (i64::from(wanted.size.w), i64::from(wanted.size.h));
    // Which side changed more, compared without division: |ww - hw| / hw against |wh - hh| / hh.
    let width_leads = (ww - hw).abs() * hh >= (wh - hh).abs() * hw;
    let (w, h) = match width_leads {
        true => (ww, ((ww * hh) + hw / 2) / hw),
        false => (((wh * hw) + hh / 2) / hh, wh),
    };
    let (w, h) = (w.max(1), h.max(1));
    let size = LogicalSize {
        w: u32::try_from(w).unwrap_or(u32::MAX),
        h: u32::try_from(h).unwrap_or(u32::MAX),
    };
    // The left edge stays when it did not move, else the right one; the same for the top.
    let x = match wanted.origin.x == held.origin.x {
        true => i64::from(wanted.origin.x),
        false => i64::from(wanted.origin.x) + ww - w,
    };
    let y = match wanted.origin.y == held.origin.y {
        true => i64::from(wanted.origin.y),
        false => i64::from(wanted.origin.y) + wh - h,
    };
    LogicalRect::new(
        LogicalPoint {
            x: i32::try_from(x).unwrap_or(wanted.origin.x),
            y: i32::try_from(y).unwrap_or(wanted.origin.y),
        },
        size,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: i32, y: i32, w: u32, h: u32) -> LogicalRect {
        LogicalRect::new(LogicalPoint { x, y }, LogicalSize { w, h })
    }

    #[test]
    fn a_dropped_window_snaps_to_the_corner_its_centre_is_nearest() {
        let area = rect(0, 24, 1920, 1056);
        let (w, h) = (320, 180);
        let cases = [
            ("top left", rect(100, 100, w, h), rect(16, 40, w, h)),
            ("top right", rect(1500, 60, w, h), rect(1584, 40, w, h)),
            ("bottom left", rect(10, 900, w, h), rect(16, 884, w, h)),
            ("bottom right", rect(1700, 950, w, h), rect(1584, 884, w, h)),
            (
                "centre goes bottom right",
                rect(800, 500, w, h),
                rect(1584, 884, w, h),
            ),
            ("already there", rect(16, 40, w, h), rect(16, 40, w, h)),
        ];
        for (name, dropped, want) in cases {
            assert_eq!(snap_corner(dropped, area), want, "{name}");
        }
    }

    #[test]
    fn a_second_output_snaps_inside_its_own_area() {
        let area = rect(1920, 0, 1280, 720);
        assert_eq!(
            snap_corner(rect(2000, 600, 320, 180), area),
            rect(1936, 524, 320, 180)
        );
    }

    #[test]
    fn a_window_too_big_for_the_margin_sits_flush() {
        let area = rect(0, 0, 400, 300);
        assert_eq!(
            snap_corner(rect(10, 10, 400, 300), area),
            rect(0, 0, 400, 300)
        );
        assert_eq!(
            snap_corner(rect(0, 0, 390, 300), area),
            rect(5, 0, 390, 300)
        );
    }

    #[test]
    fn a_resize_keeps_the_aspect_and_the_edges_that_stayed() {
        let held = rect(100, 100, 320, 180);
        let cases = [
            ("move", rect(300, 200, 320, 180), rect(300, 200, 320, 180)),
            (
                "right edge wider",
                rect(100, 100, 480, 180),
                rect(100, 100, 480, 270),
            ),
            (
                "bottom edge taller",
                rect(100, 100, 320, 360),
                rect(100, 100, 640, 360),
            ),
            (
                "left edge wider",
                rect(0, 100, 420, 180),
                rect(0, 100, 420, 236),
            ),
            (
                "top left corner",
                rect(60, 70, 360, 210),
                rect(47, 70, 373, 210),
            ),
        ];
        for (name, wanted, want) in cases {
            assert_eq!(fit_aspect(held, wanted), want, "{name}");
        }
    }

    #[test]
    fn a_window_with_no_extent_is_left_alone() {
        let wanted = rect(0, 0, 50, 50);
        assert_eq!(fit_aspect(rect(0, 0, 0, 0), wanted), wanted);
    }

    #[test]
    fn declared_and_guessed_picture_in_picture() {
        use WindowRole::{Dialog, Document, Pip};
        let cases = [
            ("declared", Pip, "", "", Some(PipSource::Declared)),
            (
                "firefox",
                Document,
                "Picture-in-Picture",
                "firefox",
                Some(PipSource::Guessed),
            ),
            (
                "chrome",
                Document,
                "Picture in picture",
                "google-chrome",
                Some(PipSource::Guessed),
            ),
            (
                "other app",
                Document,
                "Picture-in-Picture",
                "org.quire.Mail",
                None,
            ),
            ("other title", Document, "Mail", "firefox", None),
            (
                "a dialog is not guessed",
                Dialog,
                "Picture-in-Picture",
                "firefox",
                None,
            ),
        ];
        for (name, role, title, app, want) in cases {
            assert_eq!(pip_source_of(role, title, app), want, "{name}");
        }
    }
}
