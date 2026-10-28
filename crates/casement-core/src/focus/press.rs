//! Press routing (03 §1.2's table): the compositor decides before any client sees the press.

use super::{Focus, FocusView};
use crate::ids::WindowId;
use crate::input::Origin;
use casement_protocols::vocab::HeldMods;

/// A press on a window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Press {
    /// The window.
    pub window: WindowId,
    /// Which part of it.
    pub part: HitPart,
    /// The modifiers held (the Mac view, before remapping).
    pub mods: HeldMods,
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

/// How `press` is routed, given who is focused and the windows in `view`: 03 §1.2's table,
/// top row first.
///
/// | The window | Modifiers | Part | Route |
/// | --- | --- | --- | --- |
/// | has an open modal sheet | any | any | [`PressRoute::SwallowWithBell`] |
/// | is the key window | any | any | [`PressRoute::Deliver`] |
/// | is not key | Command | titlebar | [`PressRoute::MoveWithoutActivation`] |
/// | is not key | Command | anything else | [`PressRoute::DeliverWithoutActivation`] |
/// | is not key | none of Command | content | [`PressRoute::ActivateRaiseSwallow`] |
/// | is not key | none of Command | first-mouse region, titlebar, edge | [`PressRoute::ActivateRaiseDeliver`] |
///
/// The titlebar and the resize edges are the compositor's own chrome, so a first click on
/// them is a first-mouse region (03 §1.2). A press on a window the view does not hold is
/// delivered untouched. A press on a layer surface never reaches this function (it names a
/// window): layer surfaces always deliver and never take the keyboard.
pub fn route_press(press: &Press, focus: &Focus, view: &FocusView) -> PressRoute {
    if view.window(press.window).is_none() {
        return PressRoute::Deliver;
    }
    if view.sheet_of(press.window).is_some() {
        return PressRoute::SwallowWithBell;
    }
    if focus.key == Some(press.window) {
        return PressRoute::Deliver;
    }
    match (press.mods.holds(HeldMods::COMMAND), press.part) {
        (true, HitPart::Titlebar) => PressRoute::MoveWithoutActivation,
        (true, _) => PressRoute::DeliverWithoutActivation,
        (false, HitPart::Content) => PressRoute::ActivateRaiseSwallow,
        (false, HitPart::FirstMouse | HitPart::Titlebar | HitPart::Edge) => {
            PressRoute::ActivateRaiseDeliver
        }
    }
}
