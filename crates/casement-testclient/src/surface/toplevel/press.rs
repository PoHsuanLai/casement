//! The press serials a pointer delivered, for the requests that must answer one (move, resize,
//! window menu, popup grab, activation tokens, placement). The tests inject a button through
//! the fixture and read the serial the client was given.
//!
//! STAND-IN for the seat helpers of P0-H4 (`seat/pointer.rs`); it records only what H2's tests
//! need: enters, button serials, and the transcript lines.

use super::Host;
use crate::conn::TestClient;
use crate::surface::toplevel::Shared;
use casement_core::input::Edge;
use wayland_client::protocol::wl_pointer::{self, WlPointer};
use wayland_client::protocol::wl_seat::{self, WlSeat};
use wayland_client::{Connection, Dispatch, QueueHandle, WEnum};

/// One button the pointer delivered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PressSeen {
    /// The serial of the event.
    pub serial: u32,
    /// The button code.
    pub button: u32,
    /// Pressed or released.
    pub edge: Edge,
}

/// What a pointer was told.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct PressEvents {
    /// Every button event, in order.
    pub buttons: Vec<PressSeen>,
    /// The serials of every `enter`, in order.
    pub enters: Vec<u32>,
}

/// A pointer's log.
pub type PressLog = Shared<PressEvents>;

/// A bound seat with its pointer.
#[derive(Debug)]
pub struct BoundPointer {
    /// The seat.
    pub seat: WlSeat,
    /// The pointer.
    pub pointer: WlPointer,
    /// What it was told.
    pub log: PressLog,
}

impl BoundPointer {
    /// The serial of the newest press (not release), if any.
    pub fn last_press(&self) -> Option<u32> {
        self.log.with(|seen| {
            seen.buttons
                .iter()
                .rev()
                .find(|button| button.edge == Edge::Press)
                .map(|button| button.serial)
        })
    }
}

/// Binds `wl_seat` (version 7) and asks it for its pointer; the pointer exists once the seat's
/// capabilities arrive, so the test pumps before it injects.
pub fn bind_pointer(client: &mut TestClient) -> BoundPointer {
    let log = PressLog::default();
    let seat: WlSeat = client
        .bind(7..=7, log.clone())
        .expect("the compositor advertises wl_seat");
    let pointer = seat.get_pointer(&client.qh(), log.clone());
    BoundPointer { seat, pointer, log }
}

impl Dispatch<WlSeat, PressLog> for Host {
    fn event(
        _: &mut Self,
        _: &WlSeat,
        event: wl_seat::Event,
        _: &PressLog,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        // The seat's capabilities and name are in the transcript; nothing here reads them.
        let _ = event;
    }
}

impl Dispatch<WlPointer, PressLog> for Host {
    fn event(
        _: &mut Self,
        _: &WlPointer,
        event: wl_pointer::Event,
        log: &PressLog,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            wl_pointer::Event::Enter { serial, .. } => {
                log.with(|seen| seen.enters.push(serial));
            }
            wl_pointer::Event::Button {
                serial,
                button,
                state: pressed,
                ..
            } => {
                let edge = match pressed {
                    WEnum::Value(wl_pointer::ButtonState::Pressed) => Edge::Press,
                    _ => Edge::Release,
                };
                log.with(|seen| {
                    seen.buttons.push(PressSeen {
                        serial,
                        button,
                        edge,
                    });
                });
            }
            _ => {}
        }
    }
}
