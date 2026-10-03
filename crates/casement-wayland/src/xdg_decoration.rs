//! `zxdg_decoration_manager_v1`: who draws a toplevel's frame. Client-side is preferred at P0
//! (server-side titles come with `casement-chrome` at P3).

use crate::states::{GlobalFilter, WaylandFault};
use casement_core::Event;
use casement_core::policy::GlobalKind;
use smithay::reexports::wayland_protocols::xdg::decoration::zv1::server::zxdg_decoration_manager_v1::ZxdgDecorationManagerV1;
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

/// `toplevel` expressed `wish` (a new decoration object counts as `Unset`).
pub fn on_decoration_mode(
    part: &mut XdgDecorationPart,
    toplevel: &ToplevelSurface,
    wish: DecorationWish,
) -> Vec<Event> {
    let _ = (part, toplevel, wish);
    todo!("P0-H2: xdg_decoration::on_decoration_mode")
}

/// Tells `toplevel` who draws its frame (sent with its next configure).
pub fn apply_mode(
    part: &mut XdgDecorationPart,
    toplevel: &ToplevelSurface,
    mode: DecorationMode,
) -> Result<(), WaylandFault> {
    let _ = (part, toplevel, mode);
    todo!("P0-H2: xdg_decoration::apply_mode")
}
