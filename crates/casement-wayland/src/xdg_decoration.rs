//! `zxdg_decoration_manager_v1`: who draws a toplevel's frame. casement prefers to draw it (the
//! Mac titlebar, lights and shadow) and honours a client that insists on drawing its own.

use crate::states::{GlobalFilter, WaylandFault};
use casement_core::Event;
use casement_core::WindowId;
use casement_core::policy::GlobalKind;
use smithay::reexports::wayland_protocols::xdg::decoration::zv1::server::zxdg_decoration_manager_v1::ZxdgDecorationManagerV1;
use smithay::reexports::wayland_protocols::xdg::decoration::zv1::server::zxdg_toplevel_decoration_v1::Mode;
use smithay::reexports::wayland_server::{Dispatch, DisplayHandle, GlobalDispatch};
use smithay::wayland::GlobalData;
use smithay::wayland::shell::xdg::ToplevelSurface;
use smithay::wayland::shell::xdg::decoration::{XdgDecorationManagerGlobalData, XdgDecorationState};
use std::collections::BTreeMap;

/// `zxdg_decoration_manager_v1`.
#[derive(Debug)]
pub struct XdgDecorationPart {
    /// smithay's state.
    pub state: XdgDecorationState,
    /// What each window was last answered. A window absent from it draws its own frame.
    modes: BTreeMap<WindowId, DecorationMode>,
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
            modes: BTreeMap::new(),
        }
    }

    /// Remembers that `window` was answered `mode`.
    pub fn record(&mut self, window: WindowId, mode: DecorationMode) {
        self.modes.insert(window, mode);
    }

    /// Forgets `window` (it is gone).
    pub fn forget(&mut self, window: WindowId) {
        self.modes.remove(&window);
    }

    /// Who draws `window`'s frame: the client, until it was answered otherwise.
    ///
    /// ```
    /// use casement_wayland::xdg_decoration::{DecorationMode, DecorationWish, mode_for};
    ///
    /// assert_eq!(mode_for(DecorationWish::Unset), DecorationMode::ServerSide);
    /// ```
    pub fn mode_of(&self, window: WindowId) -> DecorationMode {
        self.modes
            .get(&window)
            .copied()
            .unwrap_or(DecorationMode::ClientSide)
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

/// What the compositor answers a wish with: it draws the frame unless the client said it
/// draws its own. A client that withdrew its wish, or never had one, gets the compositor's.
pub fn mode_for(wish: DecorationWish) -> DecorationMode {
    match wish {
        DecorationWish::ClientSide => DecorationMode::ClientSide,
        DecorationWish::ServerSide | DecorationWish::Unset => DecorationMode::ServerSide,
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

    /// Server-side unless the client insists on its own frame.
    #[test]
    fn the_compositor_draws_unless_the_client_insists() {
        const CASES: &[(&str, DecorationWish, DecorationMode)] = &[
            (
                "asks for ours",
                DecorationWish::ServerSide,
                DecorationMode::ServerSide,
            ),
            (
                "insists on its own",
                DecorationWish::ClientSide,
                DecorationMode::ClientSide,
            ),
            (
                "withdrew its wish",
                DecorationWish::Unset,
                DecorationMode::ServerSide,
            ),
        ];
        for (name, wish, want) in CASES {
            assert_eq!(mode_for(*wish), *want, "{name}");
        }
    }

    #[test]
    fn the_wire_mode_follows_ours() {
        assert_eq!(wire_mode(DecorationMode::ClientSide), Mode::ClientSide);
        assert_eq!(wire_mode(DecorationMode::ServerSide), Mode::ServerSide);
    }
}
