//! The window menu of the green light (and of `xdg_toplevel.show_window_menu`): which entries
//! it has, which are enabled for a window in a given state, and what each one does.
//!
//! The compositor draws no text and no menu: the shell draws it. This is the data the shell is
//! told (the entries and whether each can be picked) and what a pick means to the model. Pure.

use super::state::WindowState;
use crate::geometry::{LogicalPoint, LogicalRect};
use casement_protocols::vocab::{PlaceTarget, SplitSide, TileZone, Toggle, WindowEntry, WireCode};

/// Where the menu shows an entry: at the top level or inside one of its two submenus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum MenuGroup {
    /// Not in a submenu.
    Top,
    /// The "Move & Resize" submenu.
    MoveAndResize,
    /// The "Full Screen Tile" submenu.
    FullScreenTile,
}

/// What a picked entry asks of the model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum MenuAction {
    /// Enter or leave full screen on the window's own output.
    FullScreen(Toggle),
    /// Place the window (tile zone, fill, centre, return to the previous size).
    Place(PlaceTarget),
    /// Take one half of the screen in a Split View of two full-screen windows.
    SplitTile(SplitSide),
}

/// One entry and whether it is offered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuItem {
    /// The entry.
    pub entry: WindowEntry,
    /// Whether it can be picked (otherwise it is shown dimmed).
    pub enabled: bool,
}

/// The words the menu shows, as the Mac words them.
pub fn entry_label(entry: WindowEntry) -> &'static str {
    match entry {
        WindowEntry::EnterFullScreen => "Enter Full Screen",
        WindowEntry::ExitFullScreen => "Exit Full Screen",
        WindowEntry::Left => "Left",
        WindowEntry::Right => "Right",
        WindowEntry::Top => "Top",
        WindowEntry::Bottom => "Bottom",
        WindowEntry::TopLeft => "Top Left",
        WindowEntry::TopRight => "Top Right",
        WindowEntry::BottomLeft => "Bottom Left",
        WindowEntry::BottomRight => "Bottom Right",
        WindowEntry::ReturnToPreviousSize => "Return to Previous Size",
        WindowEntry::TileLeftOfScreen => "Left of Screen",
        WindowEntry::TileRightOfScreen => "Right of Screen",
        WindowEntry::Fill => "Fill",
        WindowEntry::Centre => "Center",
    }
}

/// Where the menu shows it.
pub fn entry_group(entry: WindowEntry) -> MenuGroup {
    match entry {
        WindowEntry::Left
        | WindowEntry::Right
        | WindowEntry::Top
        | WindowEntry::Bottom
        | WindowEntry::TopLeft
        | WindowEntry::TopRight
        | WindowEntry::BottomLeft
        | WindowEntry::BottomRight
        | WindowEntry::ReturnToPreviousSize => MenuGroup::MoveAndResize,
        WindowEntry::TileLeftOfScreen | WindowEntry::TileRightOfScreen => MenuGroup::FullScreenTile,
        _ => MenuGroup::Top,
    }
}

/// What picking it asks of the model.
pub fn entry_action(entry: WindowEntry) -> MenuAction {
    match entry {
        WindowEntry::EnterFullScreen => MenuAction::FullScreen(Toggle::On),
        WindowEntry::ExitFullScreen => MenuAction::FullScreen(Toggle::Off),
        WindowEntry::Left => MenuAction::Place(PlaceTarget::Left),
        WindowEntry::Right => MenuAction::Place(PlaceTarget::Right),
        WindowEntry::Top => MenuAction::Place(PlaceTarget::Top),
        WindowEntry::Bottom => MenuAction::Place(PlaceTarget::Bottom),
        WindowEntry::TopLeft => MenuAction::Place(PlaceTarget::TopLeft),
        WindowEntry::TopRight => MenuAction::Place(PlaceTarget::TopRight),
        WindowEntry::BottomLeft => MenuAction::Place(PlaceTarget::BottomLeft),
        WindowEntry::BottomRight => MenuAction::Place(PlaceTarget::BottomRight),
        WindowEntry::ReturnToPreviousSize => MenuAction::Place(PlaceTarget::Restore),
        WindowEntry::TileLeftOfScreen => MenuAction::SplitTile(SplitSide::Left),
        WindowEntry::TileRightOfScreen => MenuAction::SplitTile(SplitSide::Right),
        WindowEntry::Fill => MenuAction::Place(PlaceTarget::Fill),
        WindowEntry::Centre => MenuAction::Place(PlaceTarget::Centre),
    }
}

/// The tile zone this entry fills, when it is one of the eight.
fn entry_zone(entry: WindowEntry) -> Option<TileZone> {
    match entry_action(entry) {
        MenuAction::Place(target) => target.zone(),
        _ => None,
    }
}

/// The menu for a window in `state`: Enter Full Screen (Exit Full Screen when it is full
/// screen) first, then the rest in order. A window in full screen or in the dock can only leave
/// full screen; a tile zone it already fills, Fill when it is zoomed, and Return to Previous
/// Size when it is in no place to return from, are dimmed.
///
/// ```
/// use casement_core::geometry::LogicalSize;
/// use casement_core::window::{WindowState, menu_for};
/// use casement_protocols::vocab::WindowEntry;
///
/// let items = menu_for(&WindowState::New(LogicalSize { w: 10, h: 10 }), false);
/// assert_eq!(items[0].entry, WindowEntry::EnterFullScreen);
/// assert!(items.iter().any(|i| i.entry == WindowEntry::Left && i.enabled));
/// ```
pub fn menu_for(state: &WindowState, split: bool) -> Vec<MenuItem> {
    let full = matches!(state, WindowState::FullScreen { .. });
    WindowEntry::ALL
        .iter()
        .copied()
        .filter(|entry| match entry {
            WindowEntry::EnterFullScreen => !full,
            WindowEntry::ExitFullScreen => full,
            _ => true,
        })
        .map(|entry| MenuItem {
            entry,
            enabled: offered(entry, state, split),
        })
        .collect()
}

/// Whether `entry` can be picked by a window in `state`.
fn offered(entry: WindowEntry, state: &WindowState, split: bool) -> bool {
    match (state, entry) {
        (WindowState::FullScreen { .. }, WindowEntry::ExitFullScreen) => true,
        (WindowState::FullScreen { .. } | WindowState::Minimised { .. }, _) => false,
        (_, WindowEntry::ReturnToPreviousSize) => {
            matches!(
                state,
                WindowState::Tiled { .. } | WindowState::Filled { .. }
            )
        }
        (_, WindowEntry::Fill) => !matches!(state, WindowState::Filled { .. }),
        (_, WindowEntry::TileLeftOfScreen | WindowEntry::TileRightOfScreen) => split,
        (WindowState::Tiled { zone, .. }, entry) => entry_zone(entry) != Some(*zone),
        _ => true,
    }
}

/// Whether `entry` can be picked in `items`.
pub fn is_offered(items: &[MenuItem], entry: WindowEntry) -> bool {
    items.iter().any(|item| item.entry == entry && item.enabled)
}

/// The set of enabled entries as the wire sends it: bit `entry.code()` is set when it is.
///
/// ```
/// use casement_core::window::{MenuItem, enabled_mask};
/// use casement_protocols::vocab::WindowEntry;
///
/// let items = [
///     MenuItem { entry: WindowEntry::EnterFullScreen, enabled: true },
///     MenuItem { entry: WindowEntry::Left, enabled: false },
/// ];
/// assert_eq!(enabled_mask(&items), 1);
/// ```
pub fn enabled_mask(items: &[MenuItem]) -> u32 {
    items
        .iter()
        .filter(|item| item.enabled)
        .fold(0, |mask, item| mask | (1u32 << item.entry.code()))
}

/// Where the menu opens for a light at `light`: at its left edge, just under it.
///
/// ```
/// use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize};
/// use casement_core::window::menu_point;
///
/// let light = LogicalRect::new(LogicalPoint { x: 50, y: 10 }, LogicalSize { w: 12, h: 12 });
/// assert_eq!(menu_point(light), LogicalPoint { x: 50, y: 26 });
/// ```
pub fn menu_point(light: LogicalRect) -> LogicalPoint {
    LogicalPoint {
        x: light.origin.x,
        y: light
            .origin
            .y
            .saturating_add(i32::try_from(light.size.h).unwrap_or(0))
            .saturating_add(MENU_GAP),
    }
}

/// The gap between a light and the menu under it, in logical pixels.
const MENU_GAP: i32 = 4;

#[cfg(test)]
mod tests;
