//! Client dma-buf buffers backed by `/dev/udmabuf`, and the compositor's view of what it
//! imported (P0-H6).
//!
//! The ioctl that turns a memfd into a dma-buf is `unsafe`; smithay's `UdmabufAllocator` wraps
//! it behind a safe `create_buffer_from_memfd`, which is the route used here (no `unsafe` in
//! this crate, B18). The allocator's `create_buffer` is not: it over-allocates (A4).

use crate::fixture::{ClientId, Fixture};
use casement_testclient::TestClient;
use casement_testclient::misc::dmabuf_buffer::{DmabufBuffer, PlaneSpec, import_dmabuf};
use casement_testclient::udmabuf::UdmabufSource;
use smithay::backend::allocator::dmabuf::Dmabuf;
use smithay::backend::allocator::udmabuf::UdmabufAllocator;
use smithay::backend::allocator::{Fourcc, Modifier};
use smithay::reexports::wayland_server::protocol::wl_buffer::WlBuffer;
use smithay::wayland::dmabuf::get_dmabuf;
use std::os::fd::AsFd;

/// Why a client dma-buf could not be made.
#[derive(Debug, thiserror::Error)]
pub enum UdmabufBufferError {
    /// `/dev/udmabuf` refused (not bound into the jail, or the stride is not 256-aligned).
    #[error("udmabuf: {0}")]
    Udmabuf(#[from] std::io::Error),
    /// The buffer has no plane to send.
    #[error("the dma-buf has no plane")]
    NoPlane,
    /// The client could not bind `zwp_linux_dmabuf_v1`.
    #[error("client: {0}")]
    Client(#[from] casement_testclient::ClientError),
}

/// Sends `client` a linear ARGB8888 `wl_buffer` over a dma-buf made from `source` with
/// `zwp_linux_buffer_params_v1.create_immed`. The source's stride must be a multiple of 256 (a
/// width that is a multiple of 64 at four bytes a pixel).
pub fn client_buffer(
    client: &mut TestClient,
    source: &UdmabufSource,
) -> Result<DmabufBuffer, UdmabufBufferError> {
    let layout = source.layout();
    let dmabuf = UdmabufAllocator::new()?.create_buffer_from_memfd(
        source.memfd(),
        0,
        usize::try_from(layout.length).unwrap_or(usize::MAX),
        Fourcc::Argb8888,
        layout.size.w,
        layout.size.h,
        layout.stride,
    )?;
    let handle = dmabuf.handles().next().ok_or(UdmabufBufferError::NoPlane)?;
    let plane = PlaneSpec {
        width: i32::try_from(layout.size.w).unwrap_or(i32::MAX),
        height: i32::try_from(layout.size.h).unwrap_or(i32::MAX),
        format: Fourcc::Argb8888 as u32,
        modifier: Modifier::Linear.into(),
        stride: layout.stride,
    };
    Ok(import_dmabuf(client, handle.as_fd(), plane)?)
}

impl Fixture {
    /// The dma-buf the compositor made of `buffer`, which `client` imported: smithay's
    /// [`Dmabuf`], with the node the handler tagged it with. `None` when the client has no
    /// such buffer or it is not a dma-buf.
    pub fn imported_dmabuf(&self, client: ClientId, buffer: &DmabufBuffer) -> Option<Dmabuf> {
        let display = &self.state().display;
        let server = self.server_client(client)?;
        let resource = server
            .object_from_protocol_id::<WlBuffer>(display, buffer.protocol_id())
            .ok()?;
        get_dmabuf(&resource).ok().cloned()
    }
}
