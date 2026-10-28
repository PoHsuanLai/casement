//! What a client asked of the compositor about its window (`quire_window_v1`, C50).

use crate::app::MenuSource;
use crate::geometry::LogicalRect;
use casement_protocols::vocab::{Edited, FrameMode};

/// A window's `quire_window_v1` state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowHints {
    /// Where a first click on the inactive window also reaches the app (03 A5).
    pub first_mouse: Vec<LogicalRect>,
    /// How its frame is drawn.
    pub frame: FrameHint,
    /// Whether its client has said how its frame is drawn (has sent `set_frame` and not forgotten
    /// it): [`WindowHints::frame`] then holds what.
    pub frame_said: bool,
    /// Whether its document has unsaved changes.
    pub edited: Edited,
    /// Its accessibility tree's root.
    pub a11y: Option<A11yRoot>,
    /// Where its global menu lives, when the app is not ours (a Qt or GTK 3 client).
    pub menu: MenuSource,
}

/// How a window's frame is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameHint {
    /// Who draws the frame.
    pub mode: FrameMode,
    /// The window's corner radius, in logical pixels, for the shadow and the clip.
    pub corner_radius: u32,
}

/// What a window's client wants of its frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameChoice {
    /// It has not said.
    Silent,
    /// It draws its own frame, shadow and corners.
    Own,
    /// It wants casement's shadow and its corners clipped at this radius, in logical pixels.
    Shadowed {
        /// The corner radius.
        radius: u32,
    },
}

/// Where a window's accessibility tree lives: an AT-SPI object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct A11yRoot {
    /// The bus name.
    pub bus_name: String,
    /// The object path.
    pub object_path: String,
}

impl WindowHints {
    /// What a window has before its client says anything: no first-mouse regions, a client
    /// frame with square corners, saved, no accessibility root.
    pub fn unset() -> WindowHints {
        WindowHints {
            first_mouse: Vec::new(),
            frame: FrameHint {
                mode: FrameMode::Client,
                corner_radius: 0,
            },
            frame_said: false,
            edited: Edited::Saved,
            a11y: None,
            menu: MenuSource::None,
        }
    }

    /// What the client wants of its frame.
    pub fn frame_choice(&self) -> FrameChoice {
        match (self.frame_said, self.frame.mode) {
            (false, _) => FrameChoice::Silent,
            (true, FrameMode::Client) => FrameChoice::Own,
            (true, FrameMode::CompositorShadow) => FrameChoice::Shadowed {
                radius: self.frame.corner_radius,
            },
        }
    }
}
