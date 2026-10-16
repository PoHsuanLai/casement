//! ext and wlr data control: a clipboard manager. Filled by P0-H5.
//!
//! A manager that asks to be told of selections; the events reach the transcript through the
//! connection's tap.

use crate::conn::{ClientError, ClientState, TestClient};
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_client::{Connection, Dispatch, QueueHandle, event_created_child};
use wayland_protocols::ext::data_control::v1::client::ext_data_control_device_v1::{
    self, ExtDataControlDeviceV1,
};
use wayland_protocols::ext::data_control::v1::client::ext_data_control_manager_v1::{
    self, ExtDataControlManagerV1,
};
use wayland_protocols::ext::data_control::v1::client::ext_data_control_offer_v1::{
    self, ExtDataControlOfferV1,
};
use wayland_protocols::ext::data_control::v1::client::ext_data_control_source_v1::{
    self, ExtDataControlSourceV1,
};

/// The user data of every object of the manager: nothing, the transcript has the events.
#[derive(Debug, Clone, Copy, Default)]
pub struct ControlPlain;

/// A bound `ext_data_control_manager_v1` and the device it made for a seat.
#[derive(Debug)]
pub struct ExtControl {
    manager: ExtDataControlManagerV1,
    device: ExtDataControlDeviceV1,
}

impl ExtControl {
    /// Binds the manager and gets the data-control device of `seat`.
    pub fn bind(client: &TestClient, seat: &WlSeat) -> Result<ExtControl, ClientError> {
        let manager: ExtDataControlManagerV1 = client.bind(1..=1, ControlPlain)?;
        let device = manager.get_data_device(seat, &client.qh(), ControlPlain);
        let _ = client.connection().flush();
        Ok(ExtControl { manager, device })
    }

    /// The device.
    pub fn device(&self) -> &ExtDataControlDeviceV1 {
        &self.device
    }

    /// The manager.
    pub fn manager(&self) -> &ExtDataControlManagerV1 {
        &self.manager
    }

    /// Makes a data source on the manager: what a clipboard manager does to set the
    /// clipboard. The compositor answers `cancelled` when it refuses it.
    pub fn source(&self, client: &TestClient) -> ExtDataControlSourceV1 {
        let source = self.manager.create_data_source(&client.qh(), ControlPlain);
        let _ = client.connection().flush();
        source
    }
}

impl Dispatch<ExtDataControlSourceV1, ControlPlain> for ClientState {
    fn event(
        _: &mut Self,
        _: &ExtDataControlSourceV1,
        _: ext_data_control_source_v1::Event,
        _: &ControlPlain,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ExtDataControlManagerV1, ControlPlain> for ClientState {
    fn event(
        _: &mut Self,
        _: &ExtDataControlManagerV1,
        _: ext_data_control_manager_v1::Event,
        _: &ControlPlain,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ExtDataControlOfferV1, ControlPlain> for ClientState {
    fn event(
        _: &mut Self,
        _: &ExtDataControlOfferV1,
        _: ext_data_control_offer_v1::Event,
        _: &ControlPlain,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ExtDataControlDeviceV1, ControlPlain> for ClientState {
    fn event(
        _: &mut Self,
        _: &ExtDataControlDeviceV1,
        _: ext_data_control_device_v1::Event,
        _: &ControlPlain,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }

    event_created_child!(ClientState, ExtDataControlDeviceV1, [
        ext_data_control_device_v1::EVT_DATA_OFFER_OPCODE => (ExtDataControlOfferV1, ControlPlain),
    ]);
}

/// The older `zwlr_data_control_manager_v1`, same shape.
pub mod wlr {
    use crate::conn::{ClientError, ClientState, TestClient};
    use wayland_client::protocol::wl_seat::WlSeat;
    use wayland_client::{Connection, Dispatch, QueueHandle, event_created_child};
    use wayland_protocols_wlr::data_control::v1::client::zwlr_data_control_device_v1::{
        self, ZwlrDataControlDeviceV1,
    };
    use wayland_protocols_wlr::data_control::v1::client::zwlr_data_control_manager_v1::{
        self, ZwlrDataControlManagerV1,
    };
    use wayland_protocols_wlr::data_control::v1::client::zwlr_data_control_offer_v1::{
        self, ZwlrDataControlOfferV1,
    };
    use wayland_protocols_wlr::data_control::v1::client::zwlr_data_control_source_v1::{
        self, ZwlrDataControlSourceV1,
    };

    /// The user data of every object of the manager: nothing, the transcript has the events.
    #[derive(Debug, Clone, Copy, Default)]
    pub struct WlrControlPlain;

    /// A bound `zwlr_data_control_manager_v1` and the device it made for a seat.
    #[derive(Debug)]
    pub struct WlrControl {
        manager: ZwlrDataControlManagerV1,
        device: ZwlrDataControlDeviceV1,
    }

    impl WlrControl {
        /// Binds the manager and gets the data-control device of `seat`.
        pub fn bind(client: &TestClient, seat: &WlSeat) -> Result<WlrControl, ClientError> {
            let manager: ZwlrDataControlManagerV1 = client.bind(1..=2, WlrControlPlain)?;
            let device = manager.get_data_device(seat, &client.qh(), WlrControlPlain);
            let _ = client.connection().flush();
            Ok(WlrControl { manager, device })
        }

        /// The device.
        pub fn device(&self) -> &ZwlrDataControlDeviceV1 {
            &self.device
        }

        /// The manager.
        pub fn manager(&self) -> &ZwlrDataControlManagerV1 {
            &self.manager
        }

        /// Makes a data source on the manager: what a clipboard manager does to set the
        /// clipboard. The compositor answers `cancelled` when it refuses it.
        pub fn source(&self, client: &TestClient) -> ZwlrDataControlSourceV1 {
            let source = self
                .manager
                .create_data_source(&client.qh(), WlrControlPlain);
            let _ = client.connection().flush();
            source
        }
    }

    impl Dispatch<ZwlrDataControlSourceV1, WlrControlPlain> for ClientState {
        fn event(
            _: &mut Self,
            _: &ZwlrDataControlSourceV1,
            _: zwlr_data_control_source_v1::Event,
            _: &WlrControlPlain,
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
        }
    }

    impl Dispatch<ZwlrDataControlManagerV1, WlrControlPlain> for ClientState {
        fn event(
            _: &mut Self,
            _: &ZwlrDataControlManagerV1,
            _: zwlr_data_control_manager_v1::Event,
            _: &WlrControlPlain,
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
        }
    }

    impl Dispatch<ZwlrDataControlOfferV1, WlrControlPlain> for ClientState {
        fn event(
            _: &mut Self,
            _: &ZwlrDataControlOfferV1,
            _: zwlr_data_control_offer_v1::Event,
            _: &WlrControlPlain,
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
        }
    }

    impl Dispatch<ZwlrDataControlDeviceV1, WlrControlPlain> for ClientState {
        fn event(
            _: &mut Self,
            _: &ZwlrDataControlDeviceV1,
            _: zwlr_data_control_device_v1::Event,
            _: &WlrControlPlain,
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
        }

        event_created_child!(ClientState, ZwlrDataControlDeviceV1, [
            zwlr_data_control_device_v1::EVT_DATA_OFFER_OPCODE => (ZwlrDataControlOfferV1, WlrControlPlain),
        ]);
    }
}
