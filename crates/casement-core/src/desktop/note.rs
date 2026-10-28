//! What the model tells the protocol layer about the desktop: which object changed, so
//! `quire_desktop_v1` clients are sent it again (the `Effect::Desktop` of this crate).

use crate::app::AppKey;
use crate::ids::WindowId;
use porter_core::SpaceId;

/// Something about the desktop changed; the note names the object, the protocol layer reads
/// its new state from the model and sends the difference to every bound manager.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DesktopNote {
    /// An object appeared or changed. An app: identity, life, visibility, attention, recency.
    /// A window: role, state, place, output, geometry, parent, emphasis, placement mark,
    /// responsiveness. A Space: kind, position, output, activity.
    Changed(DesktopRef),
    /// The active app or the key window changed.
    Active,
    /// An object is gone.
    Removed(DesktopRef),
}

/// An object `quire_desktop_v1` clients have.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DesktopRef {
    /// An app.
    App(AppKey),
    /// A window.
    Window(WindowId),
    /// A Space.
    Space(SpaceId),
}
