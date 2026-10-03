//! The smaller protocols the stand-in serves beyond the six wlcs' descriptor lists, because
//! smithay's anvil serves them and the anvil-parity list (A6) contains their tests: viewporter,
//! fractional scale, xdg-decoration, primary selection, pointer constraints. Handler bodies
//! follow anvil's (MIT; THIRD-PARTY.md), shortened to what the wlcs tests observe.

use super::StandIn;
use smithay::input::pointer::PointerHandle;
use smithay::reexports::wayland_protocols::xdg::decoration::zv1::server::zxdg_toplevel_decoration_v1::Mode;
use smithay::reexports::wayland_server::DisplayHandle;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::wayland::compositor::with_states;
use smithay::wayland::fractional_scale::{FractionalScaleHandler, FractionalScaleManagerState, with_fractional_scale};
use smithay::wayland::pointer_constraints::{PointerConstraintsState, with_pointer_constraint};
use smithay::wayland::seat::WaylandFocus;
use smithay::wayland::selection::primary_selection::{PrimarySelectionHandler, PrimarySelectionState};
use smithay::wayland::shell::xdg::ToplevelSurface;
use smithay::wayland::shell::xdg::decoration::{XdgDecorationHandler, XdgDecorationState};
use smithay::wayland::viewporter::ViewporterState;

/// The protocol states that need no field of their own beyond being alive.
#[derive(Debug)]
pub(super) struct Extras {
    pub(super) primary_selection: PrimarySelectionState,
    _viewporter: ViewporterState,
    _decoration: XdgDecorationState,
    _fractional_scale: FractionalScaleManagerState,
    _constraints: PointerConstraintsState,
}

impl Extras {
    pub(super) fn new(dh: &DisplayHandle) -> Extras {
        Extras {
            primary_selection: PrimarySelectionState::new::<StandIn>(dh),
            _viewporter: ViewporterState::new::<StandIn>(dh),
            _decoration: XdgDecorationState::new::<StandIn>(dh),
            _fractional_scale: FractionalScaleManagerState::new::<StandIn>(dh),
            _constraints: PointerConstraintsState::new::<StandIn>(dh),
        }
    }
}

impl PrimarySelectionHandler for StandIn {
    fn primary_selection_state(&mut self) -> &mut PrimarySelectionState {
        &mut self.extras.primary_selection
    }
}

impl XdgDecorationHandler for StandIn {
    fn new_decoration(&mut self, toplevel: ToplevelSurface) {
        toplevel.with_pending_state(|state| state.decoration_mode = Some(Mode::ClientSide));
    }

    fn request_mode(&mut self, toplevel: ToplevelSurface, mode: Mode) {
        let mode = match mode {
            Mode::ServerSide => Mode::ServerSide,
            _ => Mode::ClientSide,
        };
        toplevel.with_pending_state(|state| state.decoration_mode = Some(mode));
        if toplevel.is_initial_configure_sent() {
            toplevel.send_pending_configure();
        }
    }

    fn unset_mode(&mut self, toplevel: ToplevelSurface) {
        self.request_mode(toplevel, Mode::ClientSide);
    }
}

impl FractionalScaleHandler for StandIn {
    fn new_fractional_scale(&mut self, surface: WlSurface) {
        let Some(output) = self.space.outputs().next().cloned() else {
            return;
        };
        with_states(&surface, |states| {
            with_fractional_scale(states, |scale| {
                scale.set_preferred_scale(output.current_scale().fractional_scale());
            });
        });
    }
}

impl smithay::wayland::pointer_constraints::PointerConstraintsHandler for StandIn {
    fn new_constraint(&mut self, surface: &WlSurface, pointer: &PointerHandle<Self>) {
        let focused = pointer
            .current_focus()
            .and_then(|f| f.wl_surface().map(|s| s.into_owned()));
        if focused.as_ref() == Some(surface) {
            with_pointer_constraint(surface, pointer, |constraint| {
                if let Some(constraint) = constraint {
                    constraint.activate();
                }
            });
        }
    }
}
