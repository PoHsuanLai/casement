//! What a window-list client (`zwlr_foreign_toplevel_handle_v1`) asks of a window.

use crate::geometry::LogicalRect;
use crate::ids::OutputKey;
use casement_protocols::vocab::Toggle;

/// One request on a listed window.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum WlrToplevelRequest {
    /// Maximise or restore.
    Maximise(Toggle),
    /// Minimise or bring back.
    Minimise(Toggle),
    /// Full screen on or off, optionally on one output.
    FullScreen {
        /// On or off.
        toggle: Toggle,
        /// The output it asked for.
        output: Option<OutputKey>,
    },
    /// Activate (the dock or switcher the person clicked in).
    Activate,
    /// Close.
    Close,
    /// Where the window's minimise animation should go: the dock tile's rectangle (the
    /// protocol's `set_rectangle`, in global logical coordinates once converted).
    MinimiseRect(LogicalRect),
}
