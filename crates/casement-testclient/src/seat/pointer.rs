//! wl_pointer: enter/leave, motion, buttons, axis frames with source and value120. Filled by
//! P0-H4.
//!
//! Every event reaches the transcript through the connection's tap; the helper remembers the
//! serial of the latest button press, which a drag or a move request must carry, and of the
//! latest enter, which a cursor request must carry.

use crate::conn::{ClientError, ClientState, TestClient};
use std::sync::{Arc, Mutex, PoisonError};
use wayland_client::protocol::wl_pointer::{self, WlPointer};
use wayland_client::protocol::wl_seat::{self, WlSeat};
use wayland_client::{Connection, Dispatch, QueueHandle, WEnum};

/// A bound `wl_seat` and its `wl_pointer`.
#[derive(Debug)]
pub struct Pointer {
    seat: WlSeat,
    pointer: WlPointer,
    log: PointerLog,
}

/// What the pointer's events left behind: the serial of the latest button press and of the
/// latest enter.
#[derive(Debug, Clone, Default)]
pub struct PointerLog(Arc<Mutex<Serials>>);

/// The serials a pointer was last given.
#[derive(Debug, Clone, Copy, Default)]
struct Serials {
    press: Option<u32>,
    enter: Option<u32>,
}

/// The user data of the seat object.
#[derive(Debug, Clone, Copy, Default)]
pub struct PointerPlain;

impl Pointer {
    /// Binds the seat (version 9 when offered) and asks it for its pointer.
    pub fn bind(client: &TestClient) -> Result<Pointer, ClientError> {
        let seat: WlSeat = client.bind(1..=9, PointerPlain)?;
        let log = PointerLog::default();
        let pointer = seat.get_pointer(&client.qh(), log.clone());
        let _ = client.connection().flush();
        Ok(Pointer { seat, pointer, log })
    }

    /// The serial of the latest button press this pointer received.
    pub fn press_serial(&self) -> Option<u32> {
        self.log
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .press
    }

    /// The serial of the latest `enter` this pointer received.
    pub fn enter_serial(&self) -> Option<u32> {
        self.log
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .enter
    }

    /// The `wl_pointer`.
    pub fn pointer(&self) -> &WlPointer {
        &self.pointer
    }

    /// The `wl_seat`.
    pub fn seat(&self) -> &WlSeat {
        &self.seat
    }
}

impl Dispatch<WlSeat, PointerPlain> for ClientState {
    fn event(
        _: &mut Self,
        _: &WlSeat,
        _: wl_seat::Event,
        _: &PointerPlain,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WlPointer, PointerLog> for ClientState {
    fn event(
        _: &mut Self,
        _: &WlPointer,
        event: wl_pointer::Event,
        log: &PointerLog,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let mut serials = log.0.lock().unwrap_or_else(PoisonError::into_inner);
        match event {
            wl_pointer::Event::Button { serial, state, .. }
                if state == WEnum::Value(wl_pointer::ButtonState::Pressed) =>
            {
                serials.press = Some(serial);
            }
            wl_pointer::Event::Enter { serial, .. } => serials.enter = Some(serial),
            _ => {}
        }
    }
}
