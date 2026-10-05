//! `windows.*` (03 §1.17).

use crate::config::convert;
use casement_core::settings::{
    CasementSettings, Enabled, FirstClick, MinimiseEffect, TitleDoubleClick,
};
use ds_settings::Ms;
use ds_settings::schema::Page;
use serde::{Deserialize, Serialize};

/// `windows.*`: window management.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ds_settings::SettingsSchema)]
#[serde(default)]
#[settings(file = "casement/settings.toml", domain = "windows", page = Page::Appearance)]
pub struct WindowsSection {
    /// `windows.minimise_effect`.
    #[settings(
        label = "Minimise effect",
        help = "How a window shrinks into the Dock.",
        section = "Windows"
    )]
    pub minimise_effect: MinimiseEffect,
    /// `windows.double_click_title`.
    #[settings(
        label = "Double-click a title bar to",
        help = "What a double click on a window's title bar does.",
        section = "Windows"
    )]
    pub double_click_title: TitleDoubleClick,
    /// `windows.tile_edges`.
    #[settings(
        label = "Tile by dragging to an edge",
        help = "Dragging a window to a screen edge tiles it.",
        section = "Tiling"
    )]
    pub tile_edges: Enabled,
    /// `windows.tile_top_fills`.
    #[settings(
        label = "Drag to the top to fill",
        help = "Dragging a window to the top edge fills the usable area.",
        section = "Tiling"
    )]
    pub tile_top_fills: Enabled,
    /// `windows.tile_option_drag`.
    #[settings(
        label = "Hold Option to tile",
        help = "Holding Option while dragging shows the tile zones.",
        section = "Tiling"
    )]
    pub tile_option_drag: Enabled,
    /// `windows.tile_margins`.
    #[settings(
        label = "Tiled windows have margins",
        help = "Tiled windows keep a margin around them.",
        section = "Tiling"
    )]
    pub tile_margins: Enabled,
    /// `windows.first_click`.
    #[settings(
        label = "First click on an inactive window",
        help = "Whether the click that activates a window is also delivered to it.",
        section = "Windows"
    )]
    pub first_click: FirstClick,
    /// `windows.quit_grace_ms`.
    #[settings(
        label = "Quit grace",
        help = "How long a quitting app gets before it is stopped.",
        section = "Apps",
        range = "0..=30000",
        unit = "ms",
        advanced
    )]
    pub quit_grace_ms: Ms,
    /// `windows.not_responding_ms`.
    #[settings(
        label = "Not responding after",
        help = "How long an unanswered ping takes to mark an app as not responding.",
        section = "Apps",
        range = "1000..=60000",
        unit = "ms",
        advanced
    )]
    pub not_responding_ms: Ms,
}

impl Default for WindowsSection {
    fn default() -> Self {
        convert::windows_doc(&CasementSettings::defaults().windows)
    }
}
