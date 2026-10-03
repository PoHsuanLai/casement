//! `xdg_activation_v1` (P0-H2): get a token (optionally with a press serial) and use it.

use crate::conn::TestClient;
use crate::surface::toplevel::Shared;
use crate::surface::toplevel::{Host, Toplevel};
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols::xdg::activation::v1::client::xdg_activation_token_v1::{
    self, XdgActivationTokenV1,
};
use wayland_protocols::xdg::activation::v1::client::xdg_activation_v1::XdgActivationV1;

pub use wayland_protocols::xdg::activation::v1::client::xdg_activation_token_v1::Error as TokenError;

/// The token the compositor issued, once it did.
pub type TokenText = Shared<Option<String>>;

/// A bound `xdg_activation_v1`.
#[derive(Debug)]
pub struct Activation {
    /// The global.
    pub activation: XdgActivationV1,
}

impl Activation {
    /// Binds the global (version 1).
    pub fn bind(client: &mut TestClient) -> Activation {
        Activation {
            activation: client
                .bind(1..=1, ())
                .expect("the compositor advertises xdg_activation_v1"),
        }
    }

    /// Asks for a token: with the press `serial` of `seat` when given, else a bare one. The
    /// token arrives on the next pump.
    pub fn token(
        &self,
        client: &mut TestClient,
        proof: Option<(&WlSeat, u32)>,
        app_id: Option<&str>,
    ) -> TokenText {
        let text = TokenText::default();
        let request = self
            .activation
            .get_activation_token(&client.qh(), text.clone());
        if let Some((seat, serial)) = proof {
            request.set_serial(serial, seat);
        }
        if let Some(app_id) = app_id {
            request.set_app_id(app_id.to_owned());
        }
        request.commit();
        text
    }

    /// A token request nothing has been said to yet (so a test can send `commit` twice).
    pub fn request(&self, client: &mut TestClient) -> (XdgActivationTokenV1, TokenText) {
        let text = TokenText::default();
        let request = self
            .activation
            .get_activation_token(&client.qh(), text.clone());
        (request, text)
    }

    /// Activates `toplevel` with `token`.
    pub fn activate(&self, token: &str, toplevel: &Toplevel) {
        self.activation
            .activate(token.to_owned(), toplevel.wl_surface());
    }
}

impl Dispatch<XdgActivationV1, ()> for Host {
    fn event(
        _: &mut Self,
        _: &XdgActivationV1,
        _: <XdgActivationV1 as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<XdgActivationTokenV1, TokenText> for Host {
    fn event(
        _: &mut Self,
        _: &XdgActivationTokenV1,
        event: xdg_activation_token_v1::Event,
        text: &TokenText,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let xdg_activation_token_v1::Event::Done { token } = event {
            // The token is random: the transcript names it without its text.
            text.with(|seen| *seen = Some(token));
        }
    }
}
