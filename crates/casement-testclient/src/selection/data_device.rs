//! wl_data_device: selection offers, drag and drop. Filled by P0-H5.
//!
//! The wire events reach the transcript through the connection's tap. A device built to
//! receive drops accepts the one mime type it was given on every `enter`, which is what makes
//! the compositor deliver `drop` to it.

use crate::conn::{ClientError, ClientState, TestClient};
use wayland_client::protocol::wl_data_device::{self, WlDataDevice};
use wayland_client::protocol::wl_data_device_manager::{DndAction, WlDataDeviceManager};
use wayland_client::protocol::wl_data_offer::{self, WlDataOffer};
use wayland_client::protocol::wl_data_source::{self, WlDataSource};
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::{Connection, Dispatch, QueueHandle, event_created_child};

/// What a data device does with an offer that enters one of its surfaces.
#[derive(Debug, Clone, Default)]
pub struct Receives(pub Option<String>);

/// The user data of the manager, sources and offers: nothing.
#[derive(Debug, Clone, Copy, Default)]
pub struct DragPlain;

/// A bound data device manager and the device it made.
#[derive(Debug)]
pub struct DataDevice {
    manager: WlDataDeviceManager,
    device: WlDataDevice,
}

impl DataDevice {
    /// Binds the manager and gets the data device of `seat`. With `accepts`, every offer that
    /// enters this client's surfaces is accepted for that mime type and the copy action.
    pub fn bind(
        client: &TestClient,
        seat: &WlSeat,
        accepts: Option<&str>,
    ) -> Result<DataDevice, ClientError> {
        let manager: WlDataDeviceManager = client.bind(1..=3, DragPlain)?;
        let device =
            manager.get_data_device(seat, &client.qh(), Receives(accepts.map(str::to_owned)));
        let _ = client.connection().flush();
        Ok(DataDevice { manager, device })
    }

    /// Starts a drag of `mime` data from `origin`, answering the press `serial`.
    pub fn drag(&self, client: &TestClient, origin: &WlSurface, mime: &str, serial: u32) {
        let source = self.manager.create_data_source(&client.qh(), DragPlain);
        source.offer(mime.to_owned());
        source.set_actions(DndAction::Copy);
        self.device.start_drag(Some(&source), origin, None, serial);
        let _ = client.connection().flush();
    }

    /// Sets the clipboard to `mime` data, answering the serial of the latest input.
    pub fn copy(&self, client: &TestClient, mime: &str, serial: u32) {
        let source = self.manager.create_data_source(&client.qh(), DragPlain);
        source.offer(mime.to_owned());
        self.device.set_selection(Some(&source), serial);
        let _ = client.connection().flush();
    }
}

impl Dispatch<WlDataDeviceManager, DragPlain> for ClientState {
    fn event(
        _: &mut Self,
        _: &WlDataDeviceManager,
        _: <WlDataDeviceManager as wayland_client::Proxy>::Event,
        _: &DragPlain,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WlDataSource, DragPlain> for ClientState {
    fn event(
        _: &mut Self,
        _: &WlDataSource,
        _: wl_data_source::Event,
        _: &DragPlain,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WlDataOffer, DragPlain> for ClientState {
    fn event(
        _: &mut Self,
        _: &WlDataOffer,
        _: wl_data_offer::Event,
        _: &DragPlain,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WlDataDevice, Receives> for ClientState {
    fn event(
        _: &mut Self,
        _: &WlDataDevice,
        event: wl_data_device::Event,
        receives: &Receives,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_data_device::Event::Enter {
            serial,
            id: Some(offer),
            ..
        } = event
            && let Some(mime) = &receives.0
        {
            offer.accept(serial, Some(mime.clone()));
            offer.set_actions(DndAction::Copy, DndAction::Copy);
        }
    }

    event_created_child!(ClientState, WlDataDevice, [
        wl_data_device::EVT_DATA_OFFER_OPCODE => (WlDataOffer, DragPlain),
    ]);
}
