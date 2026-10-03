//! `zxdg_decoration_manager_v1`: who draws a toplevel's frame. Client-side is preferred at P0
//! (server-side titles come with `casement-chrome` at P3).

use crate::states::{GlobalFilter, WaylandFault};
use casement_core::Event;
use casement_core::policy::GlobalKind;
use smithay::reexports::wayland_protocols::xdg::decoration::zv1::server::zxdg_decoration_manager_v1::ZxdgDecorationManagerV1;
use smithay::reexports::wayland_protocols::xdg::decoration::zv1::server::zxdg_toplevel_decoration_v1::Mode;
use smithay::reexports::wayland_server::{Dispatch, DisplayHandle, GlobalDispatch};
use smithay::wayland::GlobalData;
use smithay::wayland::shell::xdg::ToplevelSurface;
use smithay::wayland::shell::xdg::decoration::{XdgDecorationManagerGlobalData, XdgDecorationState};

/// `zxdg_decoration_manager_v1`.
#[derive(Debug)]
pub struct XdgDecorationPart {
    /// smithay's state.
    pub state: XdgDecorationState,
}

impl XdgDecorationPart {
    /// Creates the global (version 1), filtered by the I7 table.
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> XdgDecorationPart
    where
        D: GlobalDispatch<ZxdgDecorationManagerV1, XdgDecorationManagerGlobalData>
            + Dispatch<ZxdgDecorationManagerV1, GlobalData>
            + 'static,
    {
        XdgDecorationPart {
            state: XdgDecorationState::new_with_filter::<D, _>(
                display,
                role_filter.for_global(GlobalKind::XdgDecoration),
            ),
        }
    }
}

/// What a client asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DecorationWish {
    /// It draws its own frame.
    ClientSide,
    /// It asks the compositor to.
    ServerSide,
    /// It withdrew its wish.
    Unset,
}

/// Who draws the frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DecorationMode {
    /// The client.
    ClientSide,
    /// The compositor.
    ServerSide,
}

/// What the compositor answers a wish with. P0 draws no frames (server-side titles come with
/// `casement-chrome` at P3), so every wish, a server-side one included, is answered client-side.
pub fn mode_for(wish: DecorationWish) -> DecorationMode {
    match wish {
        DecorationWish::ClientSide | DecorationWish::ServerSide | DecorationWish::Unset => {
            DecorationMode::ClientSide
        }
    }
}

/// `toplevel` expressed `wish` (a new decoration object counts as `Unset`). The model has no
/// event for it: the answer is [`mode_for`] the wish, carried out by [`apply_mode`], and the
/// handler does both, so this returns nothing for the model.
pub fn on_decoration_mode(
    part: &mut XdgDecorationPart,
    toplevel: &ToplevelSurface,
    wish: DecorationWish,
) -> Vec<Event> {
    let _ = (part, toplevel, wish);
    Vec::new()
}

/// The wire's mode for ours.
fn wire_mode(mode: DecorationMode) -> Mode {
    match mode {
        DecorationMode::ClientSide => Mode::ClientSide,
        DecorationMode::ServerSide => Mode::ServerSide,
    }
}

/// Tells `toplevel` who draws its frame (sent with its next configure; a toplevel not yet
/// configured gets it with its initial one, and one already configured gets a configure now,
/// because the protocol answers every `set_mode` with one).
pub fn apply_mode(
    part: &mut XdgDecorationPart,
    toplevel: &ToplevelSurface,
    mode: DecorationMode,
) -> Result<(), WaylandFault> {
    let _ = part;
    if !toplevel.alive() {
        return Err(WaylandFault::SurfaceGone);
    }
    toplevel.with_pending_state(|state| state.decoration_mode = Some(wire_mode(mode)));
    if toplevel.is_initial_configure_sent() {
        toplevel.send_configure();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// P0 answers every wish client-side.
    #[test]
    fn every_wish_is_answered_client_side_at_p0() {
        for wish in [
            DecorationWish::ClientSide,
            DecorationWish::ServerSide,
            DecorationWish::Unset,
        ] {
            assert_eq!(mode_for(wish), DecorationMode::ClientSide, "{wish:?}");
        }
    }

    #[test]
    fn the_wire_mode_follows_ours() {
        assert_eq!(wire_mode(DecorationMode::ClientSide), Mode::ClientSide);
        assert_eq!(wire_mode(DecorationMode::ServerSide), Mode::ServerSide);
    }
}
