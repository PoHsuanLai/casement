//! `xdg_activation_v1`: tokens and activation requests. A request activates only under A7's
//! rule (the token answers a recent press of the app that has focus); the model decides with
//! `TokenCheck`. smithay stamps tokens with the wall clock (`Instant::now`), so token age is
//! judged against the session's `Mono` kept beside each token here, never smithay's stamp.

use crate::states::GlobalFilter;
use casement_core::Event;
use casement_core::time::Mono;
use smithay::reexports::wayland_protocols::xdg::activation::v1::server::xdg_activation_v1::XdgActivationV1;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::{Dispatch, DisplayHandle, GlobalDispatch};
use smithay::wayland::GlobalData;
use smithay::wayland::xdg_activation::{
    XdgActivationHandler, XdgActivationState, XdgActivationToken, XdgActivationTokenData,
};
use std::collections::BTreeMap;

/// `xdg_activation_v1`.
#[derive(Debug)]
pub struct XdgActivationPart {
    /// smithay's state.
    pub state: XdgActivationState,
    /// When each live token was issued, on the session clock.
    pub issued: BTreeMap<String, Mono>,
}

impl XdgActivationPart {
    /// Creates the global (version 1). Every connection sees it.
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> XdgActivationPart
    where
        D: GlobalDispatch<XdgActivationV1, GlobalData>
            + Dispatch<XdgActivationV1, GlobalData>
            + XdgActivationHandler
            + 'static,
    {
        let _ = role_filter;
        XdgActivationPart {
            state: XdgActivationState::new::<D>(display),
            issued: BTreeMap::new(),
        }
    }
}

/// Whether a token is issued.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenVerdict {
    /// Issue it (it is judged when used).
    Issue,
    /// Refuse it.
    Refuse,
}

/// A client asked for a token at `now`.
pub fn on_token_request(
    part: &mut XdgActivationPart,
    token: &XdgActivationToken,
    data: &XdgActivationTokenData,
    now: Mono,
) -> TokenVerdict {
    let _ = (part, token, data, now);
    todo!("P0-H2: xdg_activation::on_token_request")
}

/// `surface` asked to be activated with `token` (`Event::ToplevelRequest` with
/// `Activate(TokenCheck)`).
pub fn on_activate(
    part: &mut XdgActivationPart,
    token: &XdgActivationToken,
    data: &XdgActivationTokenData,
    surface: &WlSurface,
    now: Mono,
) -> Vec<Event> {
    let _ = (part, token, data, surface, now);
    todo!("P0-H2: xdg_activation::on_activate")
}
