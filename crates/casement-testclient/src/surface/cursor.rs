//! wp_cursor_shape_v1: a named cursor for the pointer, instead of a cursor surface. Filled by
//! P0-H4.

use crate::conn::{ClientError, ClientState, TestClient};
use wayland_client::protocol::wl_pointer::WlPointer;
use wayland_client::{Connection, Dispatch, QueueHandle};
pub use wayland_protocols::wp::cursor_shape::v1::client::wp_cursor_shape_device_v1::Shape;
use wayland_protocols::wp::cursor_shape::v1::client::wp_cursor_shape_device_v1::{
    self, WpCursorShapeDeviceV1,
};
use wayland_protocols::wp::cursor_shape::v1::client::wp_cursor_shape_manager_v1::{
    self, WpCursorShapeManagerV1,
};

/// The user data of the manager and its devices: nothing, the transcript has the events.
#[derive(Debug, Clone, Copy, Default)]
pub struct ShapePlain;

/// A bound `wp_cursor_shape_manager_v1` and the device it made for a pointer.
#[derive(Debug)]
pub struct CursorShape {
    manager: WpCursorShapeManagerV1,
    device: WpCursorShapeDeviceV1,
}

impl CursorShape {
    /// Binds the manager (version 2 when offered) and gets the shape device of `pointer`.
    pub fn bind(client: &TestClient, pointer: &WlPointer) -> Result<CursorShape, ClientError> {
        let manager: WpCursorShapeManagerV1 = client.bind(1..=2, ShapePlain)?;
        let device = manager.get_pointer(pointer, &client.qh(), ShapePlain);
        let _ = client.connection().flush();
        Ok(CursorShape { manager, device })
    }

    /// Asks for the named cursor `shape`, answering the pointer's enter `serial`.
    pub fn set_shape(&self, client: &TestClient, serial: u32, shape: Shape) {
        self.device.set_shape(serial, shape);
        let _ = client.connection().flush();
    }

    /// The manager.
    pub fn manager(&self) -> &WpCursorShapeManagerV1 {
        &self.manager
    }
}

impl Dispatch<WpCursorShapeManagerV1, ShapePlain> for ClientState {
    fn event(
        _: &mut Self,
        _: &WpCursorShapeManagerV1,
        _: wp_cursor_shape_manager_v1::Event,
        _: &ShapePlain,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WpCursorShapeDeviceV1, ShapePlain> for ClientState {
    fn event(
        _: &mut Self,
        _: &WpCursorShapeDeviceV1,
        _: wp_cursor_shape_device_v1::Event,
        _: &ShapePlain,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
