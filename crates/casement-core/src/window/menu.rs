//! The window menu of the green light (and of `xdg_toplevel.show_window_menu`): which entries
//! it has, which are enabled for a window in a given state, and what each one does.
//!
//! The compositor draws no text and no menu: the shell draws it. This is the data the shell is
//! told (the entries and whether each can be picked) and what a pick means to the model. Pure.

use super::state::WindowState;
use crate::geometry::{LogicalPoint, LogicalRect};
use casement_protocols::vocab::{PlaceTarget, TileZone, Toggle};

/// One entry of the window menu, in the order the menu lists them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum MenuEntry {
    /// Enter Full Screen.
    EnterFullScreen,
    /// Exit Full Screen (the first entry in place of Enter Full Screen, in full screen).
    ExitFullScreen,
    /// Move & Resize: the left half.
    Left,
    /// Move & Resize: the right half.
    Right,
    /// Move & Resize: the top half.
    Top,
    /// Move & Resize: the bottom half.
    Bottom,
    /// Move & Resize: the top left quarter.
    TopLeft,
    /// Move & Resize: the top right quarter.
    TopRight,
    /// Move & Resize: the bottom left quarter.
    BottomLeft,
    /// Move & Resize: the bottom right quarter.
    BottomRight,
    /// Move & Resize: Return to Previous Size.
    ReturnToPreviousSize,
    /// Full Screen Tile: Left of Screen.
    TileLeftOfScreen,
    /// Full Screen Tile: Right of Screen.
    TileRightOfScreen,
    /// Fill.
    Fill,
    /// Center.
    Centre,
}

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

/// A side of a Split View. Spelled `left` / `right` in the bindings document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SplitSide {
    /// The left of the screen.
    Left,
    /// The right of the screen.
    Right,
}

/// Whether an entry can be picked now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Offer {
    /// It can.
    Enabled,
    /// It is shown dimmed.
    Disabled,
}

/// Whether the model can put a window in a Split View.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitSupport {
    /// It can: the Full Screen Tile entries are offered.
    Available,
    /// It cannot yet: they are shown dimmed.
    Unavailable,
}

/// One entry and whether it is offered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuItem {
    /// The entry.
    pub entry: MenuEntry,
    /// Whether it can be picked.
    pub offer: Offer,
}

impl MenuEntry {
    /// Every entry in menu order.
    pub const ALL: [MenuEntry; 15] = [
        MenuEntry::EnterFullScreen,
        MenuEntry::ExitFullScreen,
        MenuEntry::Left,
        MenuEntry::Right,
        MenuEntry::Top,
        MenuEntry::Bottom,
        MenuEntry::TopLeft,
        MenuEntry::TopRight,
        MenuEntry::BottomLeft,
        MenuEntry::BottomRight,
        MenuEntry::ReturnToPreviousSize,
        MenuEntry::TileLeftOfScreen,
        MenuEntry::TileRightOfScreen,
        MenuEntry::Fill,
        MenuEntry::Centre,
    ];

    /// The words the menu shows, as the Mac words them.
    pub fn label(self) -> &'static str {
        match self {
            MenuEntry::EnterFullScreen => "Enter Full Screen",
            MenuEntry::ExitFullScreen => "Exit Full Screen",
            MenuEntry::Left => "Left",
            MenuEntry::Right => "Right",
            MenuEntry::Top => "Top",
            MenuEntry::Bottom => "Bottom",
            MenuEntry::TopLeft => "Top Left",
            MenuEntry::TopRight => "Top Right",
            MenuEntry::BottomLeft => "Bottom Left",
            MenuEntry::BottomRight => "Bottom Right",
            MenuEntry::ReturnToPreviousSize => "Return to Previous Size",
            MenuEntry::TileLeftOfScreen => "Left of Screen",
            MenuEntry::TileRightOfScreen => "Right of Screen",
            MenuEntry::Fill => "Fill",
            MenuEntry::Centre => "Center",
        }
    }

    /// Where the menu shows it.
    pub fn group(self) -> MenuGroup {
        match self {
            MenuEntry::Left
            | MenuEntry::Right
            | MenuEntry::Top
            | MenuEntry::Bottom
            | MenuEntry::TopLeft
            | MenuEntry::TopRight
            | MenuEntry::BottomLeft
            | MenuEntry::BottomRight
            | MenuEntry::ReturnToPreviousSize => MenuGroup::MoveAndResize,
            MenuEntry::TileLeftOfScreen | MenuEntry::TileRightOfScreen => MenuGroup::FullScreenTile,
            _ => MenuGroup::Top,
        }
    }

    /// What picking it asks of the model.
    pub fn action(self) -> MenuAction {
        match self {
            MenuEntry::EnterFullScreen => MenuAction::FullScreen(Toggle::On),
            MenuEntry::ExitFullScreen => MenuAction::FullScreen(Toggle::Off),
            MenuEntry::Left => MenuAction::Place(PlaceTarget::Left),
            MenuEntry::Right => MenuAction::Place(PlaceTarget::Right),
            MenuEntry::Top => MenuAction::Place(PlaceTarget::Top),
            MenuEntry::Bottom => MenuAction::Place(PlaceTarget::Bottom),
            MenuEntry::TopLeft => MenuAction::Place(PlaceTarget::TopLeft),
            MenuEntry::TopRight => MenuAction::Place(PlaceTarget::TopRight),
            MenuEntry::BottomLeft => MenuAction::Place(PlaceTarget::BottomLeft),
            MenuEntry::BottomRight => MenuAction::Place(PlaceTarget::BottomRight),
            MenuEntry::ReturnToPreviousSize => MenuAction::Place(PlaceTarget::Restore),
            MenuEntry::TileLeftOfScreen => MenuAction::SplitTile(SplitSide::Left),
            MenuEntry::TileRightOfScreen => MenuAction::SplitTile(SplitSide::Right),
            MenuEntry::Fill => MenuAction::Place(PlaceTarget::Fill),
            MenuEntry::Centre => MenuAction::Place(PlaceTarget::Centre),
        }
    }

    /// The number the wire gives it (the position in [`MenuEntry::ALL`]).
    pub fn code(self) -> u32 {
        MenuEntry::ALL
            .iter()
            .position(|entry| *entry == self)
            .and_then(|at| u32::try_from(at).ok())
            .unwrap_or(0)
    }

    /// The entry a wire number names.
    ///
    /// ```
    /// use casement_core::window::MenuEntry;
    /// assert_eq!(MenuEntry::from_code(MenuEntry::Fill.code()), Some(MenuEntry::Fill));
    /// assert_eq!(MenuEntry::from_code(99), None);
    /// ```
    pub fn from_code(code: u32) -> Option<MenuEntry> {
        usize::try_from(code)
            .ok()
            .and_then(|at| MenuEntry::ALL.get(at).copied())
    }

    /// The tile zone this entry fills, when it is one of the eight.
    fn zone(self) -> Option<TileZone> {
        match self.action() {
            MenuAction::Place(PlaceTarget::Left) => Some(TileZone::Left),
            MenuAction::Place(PlaceTarget::Right) => Some(TileZone::Right),
            MenuAction::Place(PlaceTarget::Top) => Some(TileZone::Top),
            MenuAction::Place(PlaceTarget::Bottom) => Some(TileZone::Bottom),
            MenuAction::Place(PlaceTarget::TopLeft) => Some(TileZone::TopLeft),
            MenuAction::Place(PlaceTarget::TopRight) => Some(TileZone::TopRight),
            MenuAction::Place(PlaceTarget::BottomLeft) => Some(TileZone::BottomLeft),
            MenuAction::Place(PlaceTarget::BottomRight) => Some(TileZone::BottomRight),
            _ => None,
        }
    }
}

/// The menu for a window in `state`: Enter Full Screen (Exit Full Screen when it is full
/// screen) first, then the rest in order. A window in full screen or in the dock can only leave
/// full screen; a tile zone it already fills, Fill when it is zoomed, and Return to Previous
/// Size when it is in no place to return from, are dimmed.
///
/// ```
/// use casement_core::geometry::LogicalSize;
/// use casement_core::window::{MenuEntry, Offer, SplitSupport, WindowState, menu_for};
///
/// let items = menu_for(&WindowState::New(LogicalSize { w: 10, h: 10 }), SplitSupport::Unavailable);
/// assert_eq!(items[0].entry, MenuEntry::EnterFullScreen);
/// assert!(items.iter().any(|i| i.entry == MenuEntry::Left && i.offer == Offer::Enabled));
/// ```
pub fn menu_for(state: &WindowState, split: SplitSupport) -> Vec<MenuItem> {
    let full = matches!(state, WindowState::FullScreen { .. });
    MenuEntry::ALL
        .into_iter()
        .filter(|entry| match entry {
            MenuEntry::EnterFullScreen => !full,
            MenuEntry::ExitFullScreen => full,
            _ => true,
        })
        .map(|entry| MenuItem {
            entry,
            offer: offer(entry, state, split),
        })
        .collect()
}

/// Whether `entry` can be picked by a window in `state`.
fn offer(entry: MenuEntry, state: &WindowState, split: SplitSupport) -> Offer {
    let yes = match (state, entry) {
        (WindowState::FullScreen { .. }, MenuEntry::ExitFullScreen) => true,
        (WindowState::FullScreen { .. } | WindowState::Minimised { .. }, _) => false,
        (_, MenuEntry::ReturnToPreviousSize) => {
            matches!(
                state,
                WindowState::Tiled { .. } | WindowState::Filled { .. }
            )
        }
        (_, MenuEntry::Fill) => !matches!(state, WindowState::Filled { .. }),
        (_, MenuEntry::TileLeftOfScreen | MenuEntry::TileRightOfScreen) => {
            split == SplitSupport::Available
        }
        (WindowState::Tiled { zone, .. }, entry) => entry.zone() != Some(*zone),
        _ => true,
    };
    match yes {
        true => Offer::Enabled,
        false => Offer::Disabled,
    }
}

/// Whether `entry` can be picked in `items`.
pub fn is_offered(items: &[MenuItem], entry: MenuEntry) -> bool {
    items
        .iter()
        .any(|item| item.entry == entry && item.offer == Offer::Enabled)
}

/// The set of enabled entries as the wire sends it: bit `entry.code()` is set when it is.
///
/// ```
/// use casement_core::window::{MenuEntry, Offer, MenuItem, enabled_mask};
///
/// let items = [
///     MenuItem { entry: MenuEntry::EnterFullScreen, offer: Offer::Enabled },
///     MenuItem { entry: MenuEntry::Left, offer: Offer::Disabled },
/// ];
/// assert_eq!(enabled_mask(&items), 1);
/// ```
pub fn enabled_mask(items: &[MenuItem]) -> u32 {
    items
        .iter()
        .filter(|item| item.offer == Offer::Enabled)
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
