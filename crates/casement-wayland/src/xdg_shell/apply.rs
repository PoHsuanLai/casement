//! Carrying the model's effects onto the wire: a configure, a close, a ping, a popup dismissed.

use super::XdgShellPart;
use crate::convert::size_to;
use crate::states::WaylandFault;
use casement_core::geometry::LogicalSize;
use casement_core::{
    Activation, ClientKey, ConfigureSize, ConfigureSpec, Held, PopupKey, ToplevelStates, WindowId,
};
use smithay::desktop::{PopupKind, PopupManager, find_popup_root_surface};
use smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel::State;
use smithay::utils::SERIAL_COUNTER;
use smithay::wayland::shell::PingError;
use smithay::wayland::shell::xdg::ToplevelState;

/// Every `xdg_toplevel` state a configure decides, with whether it holds. P0 floats: the model
/// never asks for tiled edges of a floating or zoomed window, and none is invented here.
pub(super) fn wire_states(states: &ToplevelStates) -> [(State, Held); 8] {
    [
        (
            State::Activated,
            match states.activation {
                Activation::Active => Held::Yes,
                Activation::Inactive => Held::No,
            },
        ),
        (State::Maximized, states.maximised),
        (State::Fullscreen, states.full_screen),
        (State::Resizing, states.resizing),
        (State::TiledTop, states.tiled.top),
        (State::TiledBottom, states.tiled.bottom),
        (State::TiledLeft, states.tiled.left),
        (State::TiledRight, states.tiled.right),
    ]
}

/// Writes `spec` into a pending toplevel state: the size to ask for (none: the client
/// chooses), the bounds, and every state set or cleared.
pub(super) fn apply_spec(state: &mut ToplevelState, spec: &ConfigureSpec) {
    state.size = match spec.size {
        ConfigureSize::Keep => None,
        ConfigureSize::Exact(size) => Some(size_to(size)),
    };
    state.bounds = (spec.bounds != LogicalSize { w: 0, h: 0 }).then(|| size_to(spec.bounds));
    for (wire, held) in wire_states(&spec.states) {
        match held {
            Held::Yes => {
                state.states.set(wire);
            }
            Held::No => {
                state.states.unset(wire);
            }
        }
    }
}

/// Sends `window` a configure. Nothing goes out when the pending state equals what the client
/// was last told; before the initial configure the state only waits in the pending slot, and
/// the initial configure (sent at the first commit) carries it.
pub fn apply_configure(
    part: &mut XdgShellPart,
    window: WindowId,
    spec: &ConfigureSpec,
) -> Result<(), WaylandFault> {
    let toplevel = part
        .toplevels
        .get(&window)
        .ok_or(WaylandFault::UnknownWindow(window))?;
    if !toplevel.alive() {
        return Err(WaylandFault::SurfaceGone);
    }
    toplevel.with_pending_state(|state| apply_spec(state, spec));
    if toplevel.is_initial_configure_sent() {
        toplevel.send_pending_configure();
    }
    Ok(())
}

/// Asks `window` to close.
pub fn apply_close(part: &mut XdgShellPart, window: WindowId) -> Result<(), WaylandFault> {
    let toplevel = part
        .toplevels
        .get(&window)
        .ok_or(WaylandFault::UnknownWindow(window))?;
    if !toplevel.alive() {
        return Err(WaylandFault::SurfaceGone);
    }
    toplevel.send_close();
    Ok(())
}

/// Pings `client`. A ping already outstanding is the answer the model is timing, so it is not
/// an error to ask again.
pub fn apply_ping(part: &mut XdgShellPart, client: ClientKey) -> Result<(), WaylandFault> {
    let shell = part.clients.get(&client).ok_or(WaylandFault::SurfaceGone)?;
    match shell.send_ping(SERIAL_COUNTER.next_serial()) {
        Ok(()) | Err(PingError::PingAlreadyPending(_)) => Ok(()),
        Err(PingError::DeadSurface) => Err(WaylandFault::SurfaceGone),
    }
}

/// Dismisses `popup` (and the popups above it).
pub fn apply_popup_done(part: &mut XdgShellPart, popup: PopupKey) -> Result<(), WaylandFault> {
    let surface = part
        .popup_surfaces
        .get(&popup)
        .ok_or(WaylandFault::SurfaceGone)?
        .clone();
    if !surface.alive() {
        return Err(WaylandFault::SurfaceGone);
    }
    let kind = PopupKind::Xdg(surface.clone());
    match find_popup_root_surface(&kind) {
        Ok(root) => {
            PopupManager::dismiss_popup(&root, &kind).map_err(|_| WaylandFault::SurfaceGone)
        }
        Err(_) => {
            surface.send_popup_done();
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use casement_core::TiledEdges;

    const NO: TiledEdges = TiledEdges {
        top: Held::No,
        bottom: Held::No,
        left: Held::No,
        right: Held::No,
    };

    fn spec(size: ConfigureSize, states: ToplevelStates) -> ConfigureSpec {
        ConfigureSpec {
            size,
            states,
            bounds: LogicalSize { w: 1920, h: 1080 },
        }
    }

    fn floating(activation: Activation) -> ToplevelStates {
        ToplevelStates {
            activation,
            maximised: Held::No,
            full_screen: Held::No,
            tiled: NO,
            resizing: Held::No,
        }
    }

    #[test]
    fn a_floating_configure_sets_activated_and_nothing_tiled() {
        let mut state = ToplevelState::default();
        apply_spec(
            &mut state,
            &spec(ConfigureSize::Keep, floating(Activation::Active)),
        );
        assert!(state.states.contains(State::Activated));
        for tiled in [
            State::TiledTop,
            State::TiledBottom,
            State::TiledLeft,
            State::TiledRight,
        ] {
            assert!(!state.states.contains(tiled), "{tiled:?}");
        }
        assert_eq!(state.size, None);
        assert_eq!(state.bounds, Some((1920, 1080).into()));
    }

    #[test]
    fn states_are_set_and_cleared_by_each_configure() {
        let mut state = ToplevelState::default();
        let zoomed = ToplevelStates {
            maximised: Held::Yes,
            resizing: Held::Yes,
            ..floating(Activation::Active)
        };
        apply_spec(
            &mut state,
            &spec(ConfigureSize::Exact(LogicalSize { w: 800, h: 600 }), zoomed),
        );
        assert!(state.states.contains(State::Maximized));
        assert!(state.states.contains(State::Resizing));
        assert_eq!(state.size, Some((800, 600).into()));
        apply_spec(
            &mut state,
            &spec(ConfigureSize::Keep, floating(Activation::Inactive)),
        );
        assert!(!state.states.contains(State::Maximized));
        assert!(!state.states.contains(State::Resizing));
        assert!(!state.states.contains(State::Activated));
        assert_eq!(state.size, None);
    }

    #[test]
    fn a_model_tile_is_carried_edge_for_edge() {
        let mut state = ToplevelState::default();
        let left = ToplevelStates {
            tiled: TiledEdges {
                top: Held::Yes,
                bottom: Held::Yes,
                left: Held::Yes,
                right: Held::No,
            },
            ..floating(Activation::Inactive)
        };
        apply_spec(&mut state, &spec(ConfigureSize::Keep, left));
        assert!(state.states.contains(State::TiledLeft));
        assert!(!state.states.contains(State::TiledRight));
    }

    #[test]
    fn zero_bounds_are_none() {
        let mut state = ToplevelState::default();
        let mut zero = spec(ConfigureSize::Keep, floating(Activation::Inactive));
        zero.bounds = LogicalSize { w: 0, h: 0 };
        apply_spec(&mut state, &zero);
        assert_eq!(state.bounds, None);
    }
}
