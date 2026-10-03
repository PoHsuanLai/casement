//! Window-management settings (`windows.*`, 03 §1.17).

use std::time::Duration;

/// `windows.*`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowSettings {
    /// `windows.minimise_effect`.
    pub minimise_effect: MinimiseEffect,
    /// `windows.double_click_title`.
    pub double_click_title: TitleDoubleClick,
    /// `windows.tile_edges`: dragging to a screen edge tiles.
    pub tile_edges: Enabled,
    /// `windows.tile_top_fills`: dragging to the top edge fills.
    pub tile_top_fills: Enabled,
    /// `windows.tile_option_drag`: Option while dragging shows the tile zones.
    pub tile_option_drag: Enabled,
    /// `windows.tile_margins`: tiled windows keep a margin.
    pub tile_margins: Enabled,
    /// `windows.first_click`.
    pub first_click: FirstClick,
    /// `windows.quit_grace_ms`: how long a quitting app gets before it is stopped.
    pub quit_grace: Duration,
    /// `windows.not_responding_ms`: how long an unanswered ping takes to mark an app.
    pub not_responding: Duration,
}

/// A switch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Enabled {
    /// On.
    On,
    /// Off.
    Off,
}

/// How a window minimises.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MinimiseEffect {
    /// The genie.
    Genie,
    /// A scale toward the tile.
    Scale,
}

/// What double-clicking a titlebar does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TitleDoubleClick {
    /// Zoom.
    Zoom,
    /// Fill the usable area.
    Fill,
    /// Minimise.
    Minimise,
    /// Nothing.
    Nothing,
}

/// What a first click on an inactive window does outside first-mouse regions (open D6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FirstClick {
    /// It only activates.
    ActivateOnly,
    /// It activates and clicks.
    ActivateAndClick,
}

impl WindowSettings {
    /// 03 §1.17's defaults.
    pub(crate) fn defaults() -> WindowSettings {
        WindowSettings {
            minimise_effect: MinimiseEffect::Genie,
            double_click_title: TitleDoubleClick::Zoom,
            tile_edges: Enabled::On,
            tile_top_fills: Enabled::On,
            tile_option_drag: Enabled::On,
            tile_margins: Enabled::On,
            first_click: FirstClick::ActivateOnly,
            quit_grace: Duration::from_millis(2000),
            not_responding: Duration::from_millis(5000),
        }
    }
}
