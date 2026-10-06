//! wl_touch: down, motion, up, frame, cancel. Every event reaches the transcript through the
//! connection's tap; the helper only binds the seat and asks it for its touch.

use crate::conn::{ClientError, ClientState, TestClient};
use wayland_client::protocol::wl_seat::{self, WlSeat};
use wayland_client::protocol::wl_touch::{self, WlTouch};
use wayland_client::{Connection, Dispatch, QueueHandle};

/// A bound `wl_seat` and its `wl_touch`.
#[derive(Debug)]
pub struct Touch {
    _seat: WlSeat,
    touch: WlTouch,
}

/// The user data of the seat and touch objects.
#[derive(Debug, Clone, Copy, Default)]
pub struct TouchPlain;

impl Touch {
    /// Binds the seat (version 9 when offered) and asks it for its touch.
    pub fn bind(client: &TestClient) -> Result<Touch, ClientError> {
        let seat: WlSeat = client.bind(1..=9, TouchPlain)?;
        let touch = seat.get_touch(&client.qh(), TouchPlain);
        let _ = client.connection().flush();
        Ok(Touch { _seat: seat, touch })
    }

    /// The `wl_touch`.
    pub fn touch(&self) -> &WlTouch {
        &self.touch
    }
}

impl Dispatch<WlSeat, TouchPlain> for ClientState {
    fn event(
        _: &mut Self,
        _: &WlSeat,
        _: wl_seat::Event,
        _: &TouchPlain,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WlTouch, TouchPlain> for ClientState {
    fn event(
        _: &mut Self,
        _: &WlTouch,
        _: wl_touch::Event,
        _: &TouchPlain,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
