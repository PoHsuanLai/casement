//! Why the arrangement changed: motion picks its spring from the cause (10 §4).

use crate::geometry::LogicalRect;
use crate::ids::OutputKey;
use ds_motion::velocity::Velocity;

/// What moved the windows, so motion can choose how they travel.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Cause {
    /// A client's own configure or commit.
    Configure,
    /// The person drags a window: it is under the hand, so it is where the pointer puts it.
    Drag,
    /// The person let go of a dragged window and it settles somewhere else (a picture in
    /// picture window to its corner).
    Drop,
    /// A keyboard command (a placement chord, a Space step).
    Keyboard,
    /// A swipe was released with this velocity.
    Swipe(SwipeRelease),
    /// Mission Control, App Exposé or Show Desktop entered or left.
    Overview,
    /// A window minimised toward a dock tile.
    Minimise(MinimiseTarget),
    /// A window opened.
    Open,
    /// A window closed.
    Close,
    /// The shown Space changed.
    SpaceSwitch,
    /// An output was plugged, unplugged or reconfigured.
    OutputChange,
}

/// The state of a released swipe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwipeRelease {
    /// How fast the fingers moved when they lifted.
    pub velocity: Velocity,
}

/// Where a minimising window goes: the dock tile sill named (`quire_shell_v1`,
/// `set_minimise_target`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MinimiseTarget {
    /// The output the tile is on.
    pub output: OutputKey,
    /// The tile, in global logical coordinates.
    pub rect: LogicalRect,
}
