//! wl_keyboard: keymap, enter with modifiers, keys, repeat info. Filled by P0-H4.
//!
//! Every event reaches the transcript through the connection's tap, so the helper only binds
//! the seat and its keyboard.

use crate::conn::{ClientError, ClientState, TestClient};
use wayland_client::protocol::wl_keyboard::{self, WlKeyboard};
use wayland_client::protocol::wl_seat::{self, WlSeat};
use wayland_client::{Connection, Dispatch, QueueHandle};

/// A bound `wl_seat` and its `wl_keyboard`.
#[derive(Debug)]
pub struct Keyboard {
    seat: WlSeat,
    keyboard: WlKeyboard,
}

/// The user data of the seat and keyboard objects: nothing, the transcript has the events.
#[derive(Debug, Clone, Copy, Default)]
pub struct KeyboardPlain;

impl Keyboard {
    /// Binds the seat (version 9 when offered) and asks it for its keyboard.
    pub fn bind(client: &TestClient) -> Result<Keyboard, ClientError> {
        let seat: WlSeat = client.bind(1..=9, KeyboardPlain)?;
        let keyboard = seat.get_keyboard(&client.qh(), KeyboardPlain);
        let _ = client.connection().flush();
        Ok(Keyboard { seat, keyboard })
    }

    /// The `wl_seat`.
    pub fn seat(&self) -> &WlSeat {
        &self.seat
    }

    /// The `wl_keyboard`.
    pub fn keyboard(&self) -> &WlKeyboard {
        &self.keyboard
    }
}

impl Dispatch<WlSeat, KeyboardPlain> for ClientState {
    fn event(
        _: &mut Self,
        _: &WlSeat,
        _: wl_seat::Event,
        _: &KeyboardPlain,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WlKeyboard, KeyboardPlain> for ClientState {
    fn event(
        _: &mut Self,
        _: &WlKeyboard,
        _: wl_keyboard::Event,
        _: &KeyboardPlain,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
