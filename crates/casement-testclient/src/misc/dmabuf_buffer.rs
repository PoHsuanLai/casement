//! A `wl_buffer` over a dma-buf the caller made, sent with `zwp_linux_buffer_params_v1.create_immed`
//! (P0-H6). Making the dma-buf is `/dev/udmabuf`'s ioctl, which needs `unsafe` or smithay's
//! wrapper of it; this crate has neither (B9, B18), so the test kit makes the dma-buf from a
//! [`crate::udmabuf::UdmabufSource`] and hands its descriptor here.

use crate::conn::{ClientError, ClientState, TestClient};
use std::os::fd::BorrowedFd;
use wayland_client::Proxy;
use wayland_client::protocol::wl_buffer::{self, WlBuffer};
use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols::wp::linux_dmabuf::zv1::client::zwp_linux_buffer_params_v1::{
    self, Flags, ZwpLinuxBufferParamsV1,
};
use wayland_protocols::wp::linux_dmabuf::zv1::client::zwp_linux_dmabuf_v1::{
    self, ZwpLinuxDmabufV1,
};

/// The user data of the global, its params and the buffers: nothing, the transcript has the
/// events.
#[derive(Debug, Clone, Copy, Default)]
pub struct DmabufPlain;

/// What one single-plane dma-buf looks like on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlaneSpec {
    /// Width in pixels.
    pub width: i32,
    /// Height in pixels.
    pub height: i32,
    /// The fourcc code.
    pub format: u32,
    /// The format modifier.
    pub modifier: u64,
    /// Bytes from one row to the next.
    pub stride: u32,
}

/// A `wl_buffer` over a dma-buf.
#[derive(Debug, Clone)]
pub struct DmabufBuffer {
    buffer: WlBuffer,
}

impl DmabufBuffer {
    /// The `wl_buffer`, to attach.
    pub fn buffer(&self) -> &WlBuffer {
        &self.buffer
    }

    /// The buffer's object id on the wire, which names it on the compositor's side.
    pub fn protocol_id(&self) -> u32 {
        self.buffer.id().protocol_id()
    }
}

/// Imports the one plane of `fd` as a `wl_buffer` (`create_immed`, version 2 and up). The
/// buffer is created at once: the compositor's answer to a refusal is a protocol error.
pub fn import_dmabuf(
    client: &mut TestClient,
    fd: BorrowedFd<'_>,
    plane: PlaneSpec,
) -> Result<DmabufBuffer, ClientError> {
    let dmabuf: ZwpLinuxDmabufV1 = client.bind(2..=6, DmabufPlain)?;
    let params = dmabuf.create_params(&client.qh(), DmabufPlain);
    // The modifier travels as two 32-bit halves.
    params.add(
        fd,
        0,
        0,
        plane.stride,
        (plane.modifier >> 32) as u32,
        plane.modifier as u32,
    );
    let buffer = params.create_immed(
        plane.width,
        plane.height,
        plane.format,
        Flags::empty(),
        &client.qh(),
        DmabufPlain,
    );
    params.destroy();
    let _ = client.connection().flush();
    Ok(DmabufBuffer { buffer })
}

impl Dispatch<ZwpLinuxDmabufV1, DmabufPlain> for ClientState {
    fn event(
        _: &mut Self,
        _: &ZwpLinuxDmabufV1,
        _: zwp_linux_dmabuf_v1::Event,
        _: &DmabufPlain,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwpLinuxBufferParamsV1, DmabufPlain> for ClientState {
    fn event(
        _: &mut Self,
        _: &ZwpLinuxBufferParamsV1,
        _: zwp_linux_buffer_params_v1::Event,
        _: &DmabufPlain,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WlBuffer, DmabufPlain> for ClientState {
    fn event(
        _: &mut Self,
        _: &WlBuffer,
        _: wl_buffer::Event,
        _: &DmabufPlain,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
