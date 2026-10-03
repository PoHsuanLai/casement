//! A window's state and what it returns to.

use crate::geometry::{LogicalRect, LogicalSize};
use casement_protocols::vocab::{PlacementMark, TileZone, WindowStateKind};
use porter_core::SpaceId;

/// Where and how a window is shown. Each state carries what leaving it restores.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowState {
    /// Just mapped at this size and not yet placed: layout puts it in the centre, then
    /// cascades (A28). It stays so until something places it.
    New(LogicalSize),
    /// Placed by hand at this rectangle.
    Floating(LogicalRect),
    /// In a tile zone; leaving it restores `restore`.
    Tiled {
        /// The zone.
        zone: TileZone,
        /// The floating rectangle to go back to.
        restore: LogicalRect,
    },
    /// Filling the usable area; leaving it restores `restore`.
    Filled {
        /// The floating rectangle to go back to.
        restore: LogicalRect,
    },
    /// Full screen on its own Space.
    FullScreen {
        /// The full-screen Space.
        space: SpaceId,
        /// The state to go back to.
        restore: Restore,
    },
    /// Minimised into the dock.
    Minimised {
        /// The state to go back to.
        restore: Restore,
    },
}

/// A state a window can return to from full screen or the dock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Restore {
    /// Unplaced, at this size.
    New(LogicalSize),
    /// Floating at this rectangle.
    Floating(LogicalRect),
    /// Tiled in this zone.
    Tiled {
        /// The zone.
        zone: TileZone,
        /// The floating rectangle behind the tile.
        restore: LogicalRect,
    },
    /// Filling the usable area.
    Filled {
        /// The floating rectangle behind the fill.
        restore: LogicalRect,
    },
}

impl WindowState {
    /// The state as the wire names it.
    pub fn kind(&self) -> WindowStateKind {
        match self {
            WindowState::New(_) | WindowState::Floating(_) => WindowStateKind::Floating,
            WindowState::Tiled { .. } => WindowStateKind::Tiled,
            WindowState::Filled { .. } => WindowStateKind::Filled,
            WindowState::FullScreen { .. } => WindowStateKind::FullScreen,
            WindowState::Minimised { .. } => WindowStateKind::Minimised,
        }
    }

    /// Where the window sits, as `quire_window_v1.placement` marks it. A minimised window
    /// carries the mark of the state it will return to.
    pub fn mark(&self) -> PlacementMark {
        match self {
            WindowState::New(_) | WindowState::Floating(_) => PlacementMark::Free,
            WindowState::Tiled { zone, .. } => zone_mark(*zone),
            WindowState::Filled { .. } => PlacementMark::Fill,
            WindowState::FullScreen { .. } => PlacementMark::FullScreen,
            WindowState::Minimised { restore } => restore.mark(),
        }
    }
}

impl WindowState {
    /// What leaving full screen or the dock goes back to, when this state can be left that
    /// way (full screen and the dock are not).
    pub fn restore(&self) -> Option<Restore> {
        match self {
            WindowState::New(size) => Some(Restore::New(*size)),
            WindowState::Floating(rect) => Some(Restore::Floating(*rect)),
            WindowState::Tiled { zone, restore } => Some(Restore::Tiled {
                zone: *zone,
                restore: *restore,
            }),
            WindowState::Filled { restore } => Some(Restore::Filled { restore: *restore }),
            WindowState::FullScreen { .. } | WindowState::Minimised { .. } => None,
        }
    }
}

impl Restore {
    /// The state this restores.
    pub fn into_state(self) -> WindowState {
        match self {
            Restore::New(size) => WindowState::New(size),
            Restore::Floating(rect) => WindowState::Floating(rect),
            Restore::Tiled { zone, restore } => WindowState::Tiled { zone, restore },
            Restore::Filled { restore } => WindowState::Filled { restore },
        }
    }

    /// The placement mark of the state this restores.
    fn mark(&self) -> PlacementMark {
        match self {
            Restore::New(_) | Restore::Floating(_) => PlacementMark::Free,
            Restore::Tiled { zone, .. } => zone_mark(*zone),
            Restore::Filled { .. } => PlacementMark::Fill,
        }
    }
}

/// The placement mark of a tile zone.
fn zone_mark(zone: TileZone) -> PlacementMark {
    match zone {
        TileZone::Left => PlacementMark::Left,
        TileZone::Right => PlacementMark::Right,
        TileZone::Top => PlacementMark::Top,
        TileZone::Bottom => PlacementMark::Bottom,
        TileZone::TopLeft => PlacementMark::TopLeft,
        TileZone::TopRight => PlacementMark::TopRight,
        TileZone::BottomLeft => PlacementMark::BottomLeft,
        TileZone::BottomRight => PlacementMark::BottomRight,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{LogicalPoint, LogicalSize};

    fn rect() -> LogicalRect {
        LogicalRect::new(LogicalPoint { x: 1, y: 2 }, LogicalSize { w: 3, h: 4 })
    }

    #[test]
    fn kinds_and_marks() {
        let space = SpaceId::parse("s-1").expect("id");
        let cases: Vec<(&str, WindowState, WindowStateKind, PlacementMark)> = vec![
            (
                "unplaced",
                WindowState::New(LogicalSize { w: 5, h: 6 }),
                WindowStateKind::Floating,
                PlacementMark::Free,
            ),
            (
                "floating",
                WindowState::Floating(rect()),
                WindowStateKind::Floating,
                PlacementMark::Free,
            ),
            (
                "tiled",
                WindowState::Tiled {
                    zone: TileZone::TopRight,
                    restore: rect(),
                },
                WindowStateKind::Tiled,
                PlacementMark::TopRight,
            ),
            (
                "filled",
                WindowState::Filled { restore: rect() },
                WindowStateKind::Filled,
                PlacementMark::Fill,
            ),
            (
                "full screen",
                WindowState::FullScreen {
                    space,
                    restore: Restore::Floating(rect()),
                },
                WindowStateKind::FullScreen,
                PlacementMark::FullScreen,
            ),
            (
                "minimised from a tile",
                WindowState::Minimised {
                    restore: Restore::Tiled {
                        zone: TileZone::Left,
                        restore: rect(),
                    },
                },
                WindowStateKind::Minimised,
                PlacementMark::Left,
            ),
            (
                "minimised from a fill",
                WindowState::Minimised {
                    restore: Restore::Filled { restore: rect() },
                },
                WindowStateKind::Minimised,
                PlacementMark::Fill,
            ),
        ];
        for (name, state, kind, mark) in cases {
            assert_eq!(state.kind(), kind, "{name}");
            assert_eq!(state.mark(), mark, "{name}");
        }
    }

    #[test]
    fn restoring_inverts_leaving() {
        let space = SpaceId::parse("s-1").expect("id");
        let leavable = [
            WindowState::New(LogicalSize { w: 5, h: 6 }),
            WindowState::Floating(rect()),
            WindowState::Tiled {
                zone: TileZone::Left,
                restore: rect(),
            },
            WindowState::Filled { restore: rect() },
        ];
        for state in leavable {
            let back = state.restore().map(Restore::into_state);
            assert_eq!(back, Some(state.clone()), "{state:?}");
        }
        let held = [
            WindowState::FullScreen {
                space,
                restore: Restore::Floating(rect()),
            },
            WindowState::Minimised {
                restore: Restore::Floating(rect()),
            },
        ];
        for state in held {
            assert_eq!(state.restore(), None, "{state:?}");
        }
    }
}
