//! zwp_tablet_manager_v2: the tablet seat of a `wl_seat` and the tablets, tools and pads it
//! announces. Filled by P0-H4.
//!
//! The wire events reach the transcript through the connection's tap; the objects the seat
//! announces are created and left alone.

use crate::conn::{ClientError, ClientState, TestClient};
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_client::{Connection, Dispatch, QueueHandle, event_created_child};
use wayland_protocols::wp::tablet::zv2::client::zwp_tablet_manager_v2::{self, ZwpTabletManagerV2};
use wayland_protocols::wp::tablet::zv2::client::zwp_tablet_pad_group_v2::{
    self, ZwpTabletPadGroupV2,
};
use wayland_protocols::wp::tablet::zv2::client::zwp_tablet_pad_ring_v2::{
    self, ZwpTabletPadRingV2,
};
use wayland_protocols::wp::tablet::zv2::client::zwp_tablet_pad_strip_v2::{
    self, ZwpTabletPadStripV2,
};
use wayland_protocols::wp::tablet::zv2::client::zwp_tablet_pad_v2::{self, ZwpTabletPadV2};
use wayland_protocols::wp::tablet::zv2::client::zwp_tablet_seat_v2::{self, ZwpTabletSeatV2};
use wayland_protocols::wp::tablet::zv2::client::zwp_tablet_tool_v2::{self, ZwpTabletToolV2};
use wayland_protocols::wp::tablet::zv2::client::zwp_tablet_v2::{self, ZwpTabletV2};

/// The user data of every object of the manager: nothing, the transcript has the events.
#[derive(Debug, Clone, Copy, Default)]
pub struct TabletPlain;

/// A bound `zwp_tablet_manager_v2` and the tablet seat it made for a `wl_seat`.
#[derive(Debug)]
pub struct TabletSeat {
    manager: ZwpTabletManagerV2,
    seat: ZwpTabletSeatV2,
}

impl TabletSeat {
    /// Binds the manager and asks it for the tablet seat of `seat`.
    pub fn bind(client: &TestClient, seat: &WlSeat) -> Result<TabletSeat, ClientError> {
        let manager: ZwpTabletManagerV2 = client.bind(1..=1, TabletPlain)?;
        let seat = manager.get_tablet_seat(seat, &client.qh(), TabletPlain);
        let _ = client.connection().flush();
        Ok(TabletSeat { manager, seat })
    }

    /// The manager.
    pub fn manager(&self) -> &ZwpTabletManagerV2 {
        &self.manager
    }

    /// The tablet seat.
    pub fn seat(&self) -> &ZwpTabletSeatV2 {
        &self.seat
    }
}

/// Dispatch that ignores every event: the transcript has them.
macro_rules! ignore_events {
    ($($proxy:ty => $module:ident),* $(,)?) => {$(
        impl Dispatch<$proxy, TabletPlain> for ClientState {
            fn event(
                _: &mut Self,
                _: &$proxy,
                _: $module::Event,
                _: &TabletPlain,
                _: &Connection,
                _: &QueueHandle<Self>,
            ) {
            }
        }
    )*};
}

ignore_events! {
    ZwpTabletManagerV2 => zwp_tablet_manager_v2,
    ZwpTabletV2 => zwp_tablet_v2,
    ZwpTabletToolV2 => zwp_tablet_tool_v2,
    ZwpTabletPadRingV2 => zwp_tablet_pad_ring_v2,
    ZwpTabletPadStripV2 => zwp_tablet_pad_strip_v2,
}

impl Dispatch<ZwpTabletSeatV2, TabletPlain> for ClientState {
    fn event(
        _: &mut Self,
        _: &ZwpTabletSeatV2,
        _: zwp_tablet_seat_v2::Event,
        _: &TabletPlain,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }

    event_created_child!(ClientState, ZwpTabletSeatV2, [
        zwp_tablet_seat_v2::EVT_TABLET_ADDED_OPCODE => (ZwpTabletV2, TabletPlain),
        zwp_tablet_seat_v2::EVT_TOOL_ADDED_OPCODE => (ZwpTabletToolV2, TabletPlain),
        zwp_tablet_seat_v2::EVT_PAD_ADDED_OPCODE => (ZwpTabletPadV2, TabletPlain),
    ]);
}

impl Dispatch<ZwpTabletPadV2, TabletPlain> for ClientState {
    fn event(
        _: &mut Self,
        _: &ZwpTabletPadV2,
        _: zwp_tablet_pad_v2::Event,
        _: &TabletPlain,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }

    event_created_child!(ClientState, ZwpTabletPadV2, [
        zwp_tablet_pad_v2::EVT_GROUP_OPCODE => (ZwpTabletPadGroupV2, TabletPlain),
    ]);
}

impl Dispatch<ZwpTabletPadGroupV2, TabletPlain> for ClientState {
    fn event(
        _: &mut Self,
        _: &ZwpTabletPadGroupV2,
        _: zwp_tablet_pad_group_v2::Event,
        _: &TabletPlain,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }

    event_created_child!(ClientState, ZwpTabletPadGroupV2, [
        zwp_tablet_pad_group_v2::EVT_RING_OPCODE => (ZwpTabletPadRingV2, TabletPlain),
        zwp_tablet_pad_group_v2::EVT_STRIP_OPCODE => (ZwpTabletPadStripV2, TabletPlain),
    ]);
}
