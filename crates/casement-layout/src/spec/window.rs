//! One window's part of the scene.

use casement_core::WindowId;
use casement_core::app::AppKey;
use casement_core::geometry::{LogicalRect, LogicalSize};
use casement_core::window::SizeLimits;
use casement_protocols::vocab::TileZone;

/// One window: where the model says it goes, its limits and its place in the stack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowSpec {
    /// The window.
    pub window: WindowId,
    /// Its app (App Exposé picks by it).
    pub app: AppKey,
    /// Where it goes.
    pub placement: WindowPlacement,
    /// Its size limits.
    pub limits: SizeLimits,
    /// Its place in its Space's stack.
    pub stack: StackIndex,
}

/// Where the model says a window goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowPlacement {
    /// At this rectangle.
    Floating(LogicalRect),
    /// At this rectangle even where it leaves the usable area: put there by hand (a test's
    /// placement, never the person's; a drag keeps a window inside). Only its size is fitted.
    Placed(LogicalRect),
    /// In this tile zone.
    Tiled(TileZone),
    /// Filling the usable area.
    Filled,
    /// Covering the whole output, the bar's strip and the dock's included.
    FullScreen,
    /// In the dock: no desktop target (motion takes the dock tile from
    /// `Cause::Minimise`); in an overview, a thumbnail at a fixed 16:9.
    Minimised,
    /// Just mapped at this size: layout places it (centre, then cascade).
    New(LogicalSize),
}

/// A window's place in its Space's stack, 0 at the bottom.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StackIndex(pub u32);
