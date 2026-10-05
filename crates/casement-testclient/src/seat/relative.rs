//! zwp_relative_pointer_v1: relative motion. Filled by P0-H4.
//!
//! The relative pointer's events reach the transcript through the connection's tap.

use crate::conn::{ClientError, ClientState, TestClient};
use crate::seat::pointer::Pointer;
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols::wp::relative_pointer::zv1::client::zwp_relative_pointer_manager_v1::ZwpRelativePointerManagerV1;
use wayland_protocols::wp::relative_pointer::zv1::client::zwp_relative_pointer_v1::{
    self, ZwpRelativePointerV1,
};

/// The user data of the objects here: nothing.
#[derive(Debug, Clone, Copy, Default)]
pub struct RelativePlain;

/// The relative pointer of one pointer.
#[derive(Debug)]
pub struct Relative {
    relative: ZwpRelativePointerV1,
}

impl Relative {
    /// Binds the manager and gets the relative pointer of `pointer`.
    pub fn bind(client: &TestClient, pointer: &Pointer) -> Result<Relative, ClientError> {
        let manager: ZwpRelativePointerManagerV1 = client.bind(1..=1, RelativePlain)?;
        let relative = manager.get_relative_pointer(pointer.pointer(), &client.qh(), RelativePlain);
        let _ = client.connection().flush();
        Ok(Relative { relative })
    }

    /// The relative pointer.
    pub fn relative(&self) -> &ZwpRelativePointerV1 {
        &self.relative
    }
}

impl Dispatch<ZwpRelativePointerManagerV1, RelativePlain> for ClientState {
    fn event(
        _: &mut Self,
        _: &ZwpRelativePointerManagerV1,
        _: <ZwpRelativePointerManagerV1 as Proxy>::Event,
        _: &RelativePlain,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwpRelativePointerV1, RelativePlain> for ClientState {
    fn event(
        _: &mut Self,
        _: &ZwpRelativePointerV1,
        _: zwp_relative_pointer_v1::Event,
        _: &RelativePlain,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
