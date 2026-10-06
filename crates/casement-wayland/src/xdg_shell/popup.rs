//! Popups: xdg_popup surfaces are the client's own (menus, tooltips, combo lists), not windows
//! of the model, so none of these functions report events. The module keeps smithay's popup
//! tree, places each popup by its positioner inside the room its output leaves, and answers
//! `reposition`. The explicit grab itself is a seat grab, taken by the handler.

use super::XdgShellPart;
use crate::client::ClientState;
use crate::convert::rect_to;
use casement_core::geometry::{LogicalPoint, LogicalRect};
use casement_core::{Event, InputSerial, PopupKey};
use smithay::desktop::PopupKind;
use smithay::reexports::wayland_server::Resource;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::utils::{Logical, Rectangle};
use smithay::wayland::compositor::with_states;
use smithay::wayland::shell::xdg::{
    PopupCachedState, PopupConfigure, PopupConfigureError, PopupSurface, PositionerState,
};

/// How much room a popup may take, in the parent surface's own coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopupBounds {
    /// No known room: the positioner is taken as asked.
    Unbounded,
    /// The popup is kept inside this rectangle, by the positioner's constraint adjustments.
    Within(LogicalRect),
}

impl PopupBounds {
    /// The room of an output (global logical) seen from a parent surface whose geometry
    /// origin sits at `parent_origin` (global logical): the same rectangle in the parent's
    /// coordinates.
    pub fn relative_to(output: LogicalRect, parent_origin: LogicalPoint) -> PopupBounds {
        PopupBounds::Within(LogicalRect::new(
            LogicalPoint {
                x: output.origin.x.saturating_sub(parent_origin.x),
                y: output.origin.y.saturating_sub(parent_origin.y),
            },
            output.size,
        ))
    }
}

/// Where the positioner puts the popup within `bounds`.
pub fn placed(positioner: &PositionerState, bounds: PopupBounds) -> Rectangle<i32, Logical> {
    match bounds {
        PopupBounds::Unbounded => positioner.get_geometry(),
        PopupBounds::Within(room) => positioner.get_unconstrained_geometry(rect_to(room)),
    }
}

/// A popup was created and given the key `popup`: it is tracked, placed within `bounds`, and
/// configured at its first commit ([`on_popup_commit`]).
pub fn on_new_popup(
    part: &mut XdgShellPart,
    surface: &PopupSurface,
    positioner: &PositionerState,
    popup: PopupKey,
    bounds: PopupBounds,
) -> Vec<Event> {
    if let Some(state) = surface
        .wl_surface()
        .client()
        .as_ref()
        .and_then(ClientState::of)
    {
        part.clients.insert(state.key, surface.client());
    }
    part.popup_surfaces.insert(popup, surface.clone());
    surface.with_pending_state(|state| {
        state.geometry = placed(positioner, bounds);
        state.positioner = *positioner;
    });
    if let Err(error) = part.popups.track_popup(PopupKind::from(surface.clone())) {
        tracing::warn!(?popup, %error, "a popup died before it could be tracked");
    }
    Vec::new()
}

/// A popup's surface committed: smithay's popup tree learns it mapped, and a popup that has not
/// yet been configured gets its initial configure.
pub fn on_popup_commit(part: &mut XdgShellPart, surface: &WlSurface) -> Vec<Event> {
    part.popups.commit(surface);
    let Some(popup) = part
        .popup_surfaces
        .values()
        .find(|held| held.wl_surface() == surface)
    else {
        return Vec::new();
    };
    if !popup.is_initial_configure_sent()
        && let Err(error) = popup.send_configure()
    {
        tracing::warn!(%error, "the initial configure of a popup was refused");
    }
    Vec::new()
}

/// A popup was destroyed.
pub fn on_popup_destroyed(part: &mut XdgShellPart, surface: &PopupSurface) -> Vec<Event> {
    part.popup_surfaces
        .retain(|_, held| held.wl_surface() != surface.wl_surface());
    Vec::new()
}

/// A popup asked for an explicit grab answering the press `serial`. The model has no part in
/// a grab (a menu is the client's); the handler took the seat grab if the press proved it.
pub fn on_popup_grab(
    part: &mut XdgShellPart,
    surface: &PopupSurface,
    serial: InputSerial,
) -> Vec<Event> {
    let _ = (part, surface, serial);
    Vec::new()
}

/// A popup asked to be repositioned (`token` goes back with the configure).
pub fn on_reposition(
    part: &mut XdgShellPart,
    surface: &PopupSurface,
    positioner: &PositionerState,
    token: u32,
    bounds: PopupBounds,
) -> Vec<Event> {
    let _ = part;
    surface.with_pending_state(|state| {
        state.geometry = placed(positioner, bounds);
        state.positioner = *positioner;
    });
    surface.send_repositioned(token);
    Vec::new()
}

/// Every live popup, for the ones that follow their parent.
pub fn popups_of(part: &XdgShellPart) -> Vec<PopupSurface> {
    part.popup_surfaces.values().cloned().collect()
}

/// The room a reactive popup is constrained by changed (its parent moved): a mapped popup whose
/// positioner is `reactive` is placed again and configured with the new position
/// (xdg_positioner.set_reactive). Any other popup stays where it is.
pub fn on_room_changed(popup: &PopupSurface, bounds: PopupBounds) {
    if !popup.is_initial_configure_sent() {
        return;
    }
    let (positioner, held) = popup.with_pending_state(|state| (state.positioner, state.geometry));
    if !positioner.reactive {
        return;
    }
    let wanted = placed(&positioner, bounds);
    if wanted == held {
        return;
    }
    popup.with_pending_state(|state| state.geometry = wanted);
    // A popup that has not mapped yet is refused (smithay asks for the committed positioner);
    // it takes the new place with its first buffer, and is not an error.
    match popup.send_configure() {
        Err(PopupConfigureError::NotReactive) | Ok(_) => {}
        Err(error) => tracing::warn!(%error, "a reactive popup could not be configured"),
    }
}

/// A mapped popup's acked configure takes effect at once, not at the next commit: "the client
/// must acknowledge the new popup configuration for the new position to take effect"
/// (xdg_popup.repositioned). Before the first commit the ack waits for it as ever.
pub fn on_popup_acked(surface: &WlSurface, configure: PopupConfigure) {
    with_states(surface, |states| {
        let mut cached = states.cached_state.get::<PopupCachedState>();
        let current = cached.current();
        if current.last_acked.is_some() {
            current.last_acked = Some(configure);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use casement_core::geometry::LogicalSize;
    use smithay::reexports::wayland_protocols::xdg::shell::server::xdg_positioner::{
        Anchor, ConstraintAdjustment, Gravity,
    };

    fn menu(at: (i32, i32), adjustment: ConstraintAdjustment) -> PositionerState {
        PositionerState {
            rect_size: (100, 50).into(),
            anchor_rect: Rectangle::new(at.into(), (10, 10).into()),
            anchor_edges: Anchor::TopLeft,
            gravity: Gravity::BottomRight,
            constraint_adjustment: adjustment,
            ..PositionerState::default()
        }
    }

    fn room() -> PopupBounds {
        PopupBounds::Within(LogicalRect::new(
            LogicalPoint { x: 0, y: 0 },
            LogicalSize { w: 1000, h: 800 },
        ))
    }

    /// The positioner's own answer when it fits; slid back inside when it does not and the
    /// client allowed it; as asked when the client allowed nothing.
    #[test]
    fn a_popup_is_kept_in_the_room_the_client_allows() {
        let slide = ConstraintAdjustment::SlideX | ConstraintAdjustment::SlideY;
        let rows = [
            ("fits", menu((100, 100), slide), (100, 100)),
            ("slides left", menu((950, 100), slide), (900, 100)),
            ("slides up", menu((100, 790), slide), (100, 750)),
            (
                "may not adjust",
                menu((950, 100), ConstraintAdjustment::empty()),
                (950, 100),
            ),
        ];
        for (case, positioner, at) in rows {
            let got = placed(&positioner, room());
            assert_eq!((got.loc.x, got.loc.y), at, "{case}");
            assert_eq!((got.size.w, got.size.h), (100, 50), "{case}");
        }
    }

    #[test]
    fn no_known_room_takes_the_positioner_as_asked() {
        let slide = ConstraintAdjustment::SlideX;
        let got = placed(&menu((950, 100), slide), PopupBounds::Unbounded);
        assert_eq!((got.loc.x, got.loc.y), (950, 100));
    }

    #[test]
    fn an_output_is_seen_from_its_parents_origin() {
        let output = LogicalRect::new(
            LogicalPoint { x: 1920, y: 0 },
            LogicalSize { w: 1920, h: 1080 },
        );
        let from = PopupBounds::relative_to(output, LogicalPoint { x: 2000, y: 40 });
        assert_eq!(
            from,
            PopupBounds::Within(LogicalRect::new(
                LogicalPoint { x: -80, y: -40 },
                LogicalSize { w: 1920, h: 1080 }
            ))
        );
    }
}
