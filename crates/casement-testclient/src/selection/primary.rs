//! zwp_primary_selection_device_v1: the middle-click selection. Filled by P0-H5.
//!
//! The wire events reach the transcript through the connection's tap.

use crate::conn::{ClientError, ClientState, TestClient};
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_client::{Connection, Dispatch, QueueHandle, event_created_child};
use wayland_protocols::wp::primary_selection::zv1::client::zwp_primary_selection_device_manager_v1::{
    self, ZwpPrimarySelectionDeviceManagerV1,
};
use wayland_protocols::wp::primary_selection::zv1::client::zwp_primary_selection_device_v1::{
    self, ZwpPrimarySelectionDeviceV1,
};
use wayland_protocols::wp::primary_selection::zv1::client::zwp_primary_selection_offer_v1::{
    self, ZwpPrimarySelectionOfferV1,
};
use wayland_protocols::wp::primary_selection::zv1::client::zwp_primary_selection_source_v1::{
    self, ZwpPrimarySelectionSourceV1,
};

/// The user data of every object of the manager: nothing, the transcript has the events.
#[derive(Debug, Clone, Copy, Default)]
pub struct PrimaryPlain;

/// A bound primary-selection manager and the device it made for a seat.
#[derive(Debug)]
pub struct PrimaryDevice {
    manager: ZwpPrimarySelectionDeviceManagerV1,
    device: ZwpPrimarySelectionDeviceV1,
}

impl PrimaryDevice {
    /// Binds the manager and gets the device of `seat`.
    pub fn bind(client: &TestClient, seat: &WlSeat) -> Result<PrimaryDevice, ClientError> {
        let manager: ZwpPrimarySelectionDeviceManagerV1 = client.bind(1..=1, PrimaryPlain)?;
        let device = manager.get_device(seat, &client.qh(), PrimaryPlain);
        let _ = client.connection().flush();
        Ok(PrimaryDevice { manager, device })
    }

    /// Sets the primary selection to `mime` data, answering `serial`.
    pub fn select(&self, client: &TestClient, mime: &str, serial: u32) {
        let source = self.manager.create_source(&client.qh(), PrimaryPlain);
        source.offer(mime.to_owned());
        self.device.set_selection(Some(&source), serial);
        let _ = client.connection().flush();
    }
}

impl Dispatch<ZwpPrimarySelectionDeviceManagerV1, PrimaryPlain> for ClientState {
    fn event(
        _: &mut Self,
        _: &ZwpPrimarySelectionDeviceManagerV1,
        _: zwp_primary_selection_device_manager_v1::Event,
        _: &PrimaryPlain,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwpPrimarySelectionSourceV1, PrimaryPlain> for ClientState {
    fn event(
        _: &mut Self,
        _: &ZwpPrimarySelectionSourceV1,
        _: zwp_primary_selection_source_v1::Event,
        _: &PrimaryPlain,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwpPrimarySelectionOfferV1, PrimaryPlain> for ClientState {
    fn event(
        _: &mut Self,
        _: &ZwpPrimarySelectionOfferV1,
        _: zwp_primary_selection_offer_v1::Event,
        _: &PrimaryPlain,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwpPrimarySelectionDeviceV1, PrimaryPlain> for ClientState {
    fn event(
        _: &mut Self,
        _: &ZwpPrimarySelectionDeviceV1,
        _: zwp_primary_selection_device_v1::Event,
        _: &PrimaryPlain,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }

    event_created_child!(ClientState, ZwpPrimarySelectionDeviceV1, [
        zwp_primary_selection_device_v1::EVT_DATA_OFFER_OPCODE => (ZwpPrimarySelectionOfferV1, PrimaryPlain),
    ]);
}
