//! One layer surface's state, as its client configured it (wlr-layer-shell, in our words).

use crate::ids::OutputKey;
use casement_protocols::vocab::ShellRole;

/// What a layer surface asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerSpec {
    /// The output it sits on.
    pub output: OutputKey,
    /// Which layer.
    pub level: LayerLevel,
    /// The edges it is held to.
    pub anchor: Anchors,
    /// The space it keeps windows out of.
    pub exclusive: ExclusiveZone,
    /// Its distance from each anchored edge.
    pub margin: Margins,
    /// Whether it takes the keyboard.
    pub keyboard: KeyboardInteractivity,
    /// The namespace its client gave (`"bar"`, `"dock"`).
    pub namespace: String,
    /// The shell role sill gave it through `quire_shell_v1`.
    pub role: ShellRoleTag,
}

/// The four layers, bottom to top.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum LayerLevel {
    /// Below everything (wallpaper).
    Background,
    /// Below windows.
    Bottom,
    /// Above windows (the bar, the dock).
    Top,
    /// Above everything, full-screen windows included.
    Overlay,
}

/// Which edges a layer surface is held to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Anchors {
    /// The top edge.
    pub top: Anchor,
    /// The bottom edge.
    pub bottom: Anchor,
    /// The left edge.
    pub left: Anchor,
    /// The right edge.
    pub right: Anchor,
}

/// Whether a layer surface is held to an edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    /// Not held.
    Free,
    /// Held.
    Held,
}

/// How a layer surface treats the space beside its anchored edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExclusiveZone {
    /// It ignores other surfaces' zones and reserves none (`-1`).
    Ignore,
    /// It avoids other zones and reserves none (`0`).
    Respect,
    /// It reserves this many logical pixels beside its edge.
    Reserve(u32),
}

/// A layer surface's distance from each edge, in logical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Margins {
    /// From the top edge.
    pub top: i32,
    /// From the right edge.
    pub right: i32,
    /// From the bottom edge.
    pub bottom: i32,
    /// From the left edge.
    pub left: i32,
}

/// Whether a layer surface takes the keyboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyboardInteractivity {
    /// Never.
    None,
    /// When clicked.
    OnDemand,
    /// Whenever it is mapped (a launcher, a lock-like overlay).
    Exclusive,
}

/// The shell role a layer surface has been given, if any.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellRoleTag {
    /// None yet.
    Unassigned,
    /// This role.
    Assigned(ShellRole),
}
