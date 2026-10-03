//! `xdg_activation_v1` (P0-H2): tokens are judged on the session clock (A36), and only a token
//! earned by a recent press activates (A7).

use crate::core_loop;
use crate::state::State;
use casement_core::ClientKey;
use casement_wayland::ClientState;
use casement_wayland::convert::serial_of;
use casement_wayland::xdg_activation::{
    PressProof, TokenAsk, TokenVerdict, on_activate, on_token_request, proof_of, use_token,
};
use smithay::reexports::wayland_server::backend::ClientId;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::wayland::xdg_activation::{
    XdgActivationHandler, XdgActivationState, XdgActivationToken, XdgActivationTokenData,
};

impl State {
    /// The key of the client behind a backend id, if casement inserted it.
    fn client_key_of(&self, id: &ClientId) -> Option<ClientKey> {
        let data = self
            .display
            .backend_handle()
            .get_client_data(id.clone())
            .ok()?;
        (*data).downcast_ref::<ClientState>().map(|state| state.key)
    }
}

impl XdgActivationHandler for State {
    fn activation_state(&mut self) -> &mut XdgActivationState {
        &mut self.wl.xdg_activation.state
    }

    fn token_created(&mut self, token: XdgActivationToken, data: XdgActivationTokenData) -> bool {
        let now = self.clock.now();
        let requester = data
            .client_id
            .as_ref()
            .and_then(|id| self.client_key_of(id));
        let proof = match (&data.serial, requester) {
            (Some((serial, _seat)), Some(client)) => {
                let record = self
                    .wl
                    .seat
                    .serials
                    .answers(serial_of(*serial), client)
                    .copied();
                proof_of(record.as_ref(), now)
            }
            _ => PressProof::Unproven,
        };
        let ask = TokenAsk { requester, proof };
        on_token_request(&mut self.wl.xdg_activation, &token, &data, now, ask)
            == TokenVerdict::Issue
    }

    fn request_activation(
        &mut self,
        token: XdgActivationToken,
        token_data: XdgActivationTokenData,
        surface: WlSurface,
    ) {
        let _ = token_data;
        let now = self.clock.now();
        let check = use_token(&mut self.wl.xdg_activation, &token, now);
        let Some(window) = self.wl.xdg_shell.window_of_surface(&surface) else {
            return;
        };
        if self.wl.xdg_shell.is_mapped(window) {
            core_loop::feed(self, on_activate(window, check));
        } else {
            // It asked before it mapped (the usual launcher flow): the map event carries it.
            self.wl.xdg_shell.stage_activation(window, check);
        }
    }
}
