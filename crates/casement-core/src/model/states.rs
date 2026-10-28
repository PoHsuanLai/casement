//! The `xdg_toplevel` states a window's record and focus imply.

use super::Model;
use crate::effect::{TiledEdges, ToplevelStates};
use crate::focus::Focus;
use crate::ids::WindowId;
use crate::window::{Window, WindowState};
use casement_protocols::vocab::TileZone;

impl Model {
    /// The states a configure to `window` carries now.
    pub fn states_of(&self, window: WindowId) -> Option<ToplevelStates> {
        self.windows
            .get(&window)
            .map(|window| toplevel_states(window, &self.focus))
    }
}

/// `window`'s states: active when it is the key window, maximised when filled, full screen,
/// tiled on the edges its zone touches, never resizing (a grab sets that).
pub(super) fn toplevel_states(window: &Window, focus: &Focus) -> ToplevelStates {
    ToplevelStates {
        activated: focus.key == Some(window.id),
        maximised: matches!(window.state, WindowState::Filled { .. }),
        full_screen: matches!(window.state, WindowState::FullScreen { .. }),
        tiled: match &window.state {
            WindowState::Tiled { zone, .. } => tiled_edges(*zone),
            _ => TiledEdges {
                top: false,
                bottom: false,
                left: false,
                right: false,
            },
        },
        resizing: false,
    }
}

/// The edges of the usable area a tile zone touches.
fn tiled_edges(zone: TileZone) -> TiledEdges {
    const Y: bool = true;
    const N: bool = false;
    let [top, bottom, left, right] = match zone {
        TileZone::Left => [Y, Y, Y, N],
        TileZone::Right => [Y, Y, N, Y],
        TileZone::Top => [Y, N, Y, Y],
        TileZone::Bottom => [N, Y, Y, Y],
        TileZone::TopLeft => [Y, N, Y, N],
        TileZone::TopRight => [Y, N, N, Y],
        TileZone::BottomLeft => [N, Y, Y, N],
        TileZone::BottomRight => [N, Y, N, Y],
    };
    TiledEdges {
        top,
        bottom,
        left,
        right,
    }
}
