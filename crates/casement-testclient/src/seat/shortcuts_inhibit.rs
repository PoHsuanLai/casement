//! zwp_keyboard_shortcuts_inhibitor_v1. Filled by P0-H4.
//!
//! The inhibitor's `active` and `inactive` events reach the transcript through the
//! connection's tap.

use crate::conn::{ClientError, ClientState, TestClient};
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols::wp::keyboard_shortcuts_inhibit::zv1::client::zwp_keyboard_shortcuts_inhibit_manager_v1::ZwpKeyboardShortcutsInhibitManagerV1;
use wayland_protocols::wp::keyboard_shortcuts_inhibit::zv1::client::zwp_keyboard_shortcuts_inhibitor_v1::{
    self, ZwpKeyboardShortcutsInhibitorV1,
};

/// The user data of the objects here: nothing.
#[derive(Debug, Clone, Copy, Default)]
pub struct InhibitPlain;

/// An inhibitor the client created.
#[derive(Debug)]
pub struct Inhibitor {
    inhibitor: ZwpKeyboardShortcutsInhibitorV1,
}

impl Inhibitor {
    /// Asks for every key of `seat` while `surface` has the keyboard.
    pub fn create(
        client: &TestClient,
        surface: &WlSurface,
        seat: &WlSeat,
    ) -> Result<Inhibitor, ClientError> {
        let manager: ZwpKeyboardShortcutsInhibitManagerV1 = client.bind(1..=1, InhibitPlain)?;
        let inhibitor = manager.inhibit_shortcuts(surface, seat, &client.qh(), InhibitPlain);
        let _ = client.connection().flush();
        Ok(Inhibitor { inhibitor })
    }

    /// The inhibitor object.
    pub fn inhibitor(&self) -> &ZwpKeyboardShortcutsInhibitorV1 {
        &self.inhibitor
    }
}

impl Dispatch<ZwpKeyboardShortcutsInhibitManagerV1, InhibitPlain> for ClientState {
    fn event(
        _: &mut Self,
        _: &ZwpKeyboardShortcutsInhibitManagerV1,
        _: <ZwpKeyboardShortcutsInhibitManagerV1 as Proxy>::Event,
        _: &InhibitPlain,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwpKeyboardShortcutsInhibitorV1, InhibitPlain> for ClientState {
    fn event(
        _: &mut Self,
        _: &ZwpKeyboardShortcutsInhibitorV1,
        _: zwp_keyboard_shortcuts_inhibitor_v1::Event,
        _: &InhibitPlain,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
