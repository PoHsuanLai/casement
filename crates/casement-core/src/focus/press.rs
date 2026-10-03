//! Press routing (03 §1.2's table): the compositor decides before any client sees the press.

use super::{Focus, FocusView};
use crate::ids::WindowId;
use crate::input::Origin;
use casement_protocols::vocab::MacMods;

/// A press on a window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Press {
    /// The window.
    pub window: WindowId,
    /// Which part of it.
    pub part: HitPart,
    /// The modifiers held (the Mac view, before remapping).
    pub mods: MacMods,
    /// Where the press came from.
    pub origin: Origin,
}

/// Which part of a window a press landed on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HitPart {
    /// Its content, outside any first-mouse region.
    Content,
    /// A region where a first click reaches the app (03 A5).
    FirstMouse,
    /// Its titlebar.
    Titlebar,
    /// A resize edge.
    Edge,
}

/// What happens to a press.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PressRoute {
    /// Deliver it (the window is key).
    Deliver,
    /// Activate the app, raise the window, deliver the press (a first-mouse region).
    ActivateRaiseDeliver,
    /// Activate and raise; swallow the press and its release.
    ActivateRaiseSwallow,
    /// Deliver without activating or raising (Command held, A6).
    DeliverWithoutActivation,
    /// Move the window without activating it (Command-drag on a titlebar, A6).
    MoveWithoutActivation,
    /// Swallow and ring the bell (the window's sheet is open).
    SwallowWithBell,
}

/// How `press` is routed, given who is focused and the windows in `view`.
pub fn route_press(press: &Press, focus: &Focus, view: &FocusView) -> PressRoute {
    let _ = (press, focus, view);
    todo!("P0-F1-focus: 03 §1.2 press routing")
}
