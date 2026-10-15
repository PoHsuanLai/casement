//! What a client asked of the compositor about its window (`quire_window_v1`, C50).

use crate::app::MenuSource;
use crate::geometry::LogicalRect;
use casement_protocols::vocab::{Edited, FrameMode};

/// A window's `quire_window_v1` state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowHints {
    /// Where a first click on the inactive window also reaches the app (03 A5).
    pub first_mouse: FirstMouse,
    /// How its frame is drawn.
    pub frame: FrameHint,
    /// Whether its document has unsaved changes.
    pub edited: Edited,
    /// Its accessibility tree's root.
    pub a11y: A11yRoot,
    /// Where its global menu lives, when the app is not ours (a Qt or GTK 3 client).
    pub menu: MenuSource,
}

/// Where a first click passes through to the app.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FirstMouse {
    /// Nowhere: a first click only activates.
    Nowhere,
    /// In these window-local rectangles.
    Regions(Vec<LogicalRect>),
}

/// How a window's frame is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameHint {
    /// Who draws the frame.
    pub mode: FrameMode,
    /// The window's corner radius, in logical pixels, for the shadow and the clip.
    pub corner_radius: u32,
}

/// Where a window's accessibility tree lives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum A11yRoot {
    /// The client has not said.
    Unknown,
    /// This AT-SPI object.
    At {
        /// The bus name.
        bus_name: String,
        /// The object path.
        object_path: String,
    },
}

impl WindowHints {
    /// What a window has before its client says anything: no first-mouse regions, a client
    /// frame with square corners, saved, no accessibility root.
    pub fn unset() -> WindowHints {
        WindowHints {
            first_mouse: FirstMouse::Nowhere,
            frame: FrameHint {
                mode: FrameMode::Client,
                corner_radius: 0,
            },
            edited: Edited::Saved,
            a11y: A11yRoot::Unknown,
            menu: MenuSource::None,
        }
    }
}
