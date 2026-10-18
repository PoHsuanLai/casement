//! Shared-memory buffers over a memfd, written with `write_all_at` (no mmap: no `unsafe`).

use crate::conn::{ClientState, TestClient};
use crate::pattern::Pattern;
use casement_core::geometry::PhysicalSize;
use rustix::fs::{MemfdFlags, ftruncate, memfd_create};
use std::fs::File;
use std::os::fd::{AsFd, OwnedFd};
use std::os::unix::fs::FileExt;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use wayland_client::protocol::wl_buffer::{self, WlBuffer};
use wayland_client::protocol::wl_shm::{self, Format, WlShm};
use wayland_client::protocol::wl_shm_pool::{self, WlShmPool};
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};

/// A `wl_buffer` over a memfd pool, holding a pattern.
#[derive(Debug)]
pub struct ShmBuffer {
    size: PhysicalSize,
    buffer: WlBuffer,
    file: File,
    released: Arc<AtomicBool>,
}

/// The user data of what `shm_buffer` creates: the buffer's release flag. Helper modules of the
/// shm protocol bind their own `wl_shm`, with their own user data, so the two never collide.
#[derive(Debug, Clone, Default)]
struct Plain(Arc<AtomicBool>);

/// A `size` ARGB8888 buffer of `client` holding `pattern`.
///
/// # Panics
///
/// When `wl_shm` is not advertised or the memfd cannot be made: a test that asks for a buffer
/// has nothing to test without one.
pub fn shm_buffer(client: &mut TestClient, size: PhysicalSize, pattern: Pattern) -> ShmBuffer {
    let released = Arc::new(AtomicBool::new(false));
    let shm: WlShm = client
        .bind(1..=2, Plain::default())
        .expect("shm_buffer: the compositor advertises wl_shm");
    let bytes = pattern.bytes(size);
    let file = File::from(memfd_sized(bytes.len()));
    file.write_all_at(&bytes, 0)
        .expect("shm_buffer: write the pattern into the memfd");
    let qh = client.qh();
    let length = i32::try_from(bytes.len()).expect("shm_buffer: a buffer under 2 GiB");
    let pool = shm.create_pool(file.as_fd(), length, &qh, Plain::default());
    let buffer = pool.create_buffer(
        0,
        i32::try_from(size.w).expect("shm_buffer: a width under 2^31"),
        i32::try_from(size.h).expect("shm_buffer: a height under 2^31"),
        i32::try_from(size.w * 4).expect("shm_buffer: a stride under 2^31"),
        Format::Argb8888,
        &qh,
        Plain(released.clone()),
    );
    // The buffer keeps its memory alive on the compositor's side; the pool and the global
    // handle are not needed again.
    pool.destroy();
    if shm.version() >= 2 {
        shm.release();
    }
    let _ = client.connection().flush();
    ShmBuffer {
        size,
        buffer,
        file,
        released,
    }
}

/// An empty memfd of `len` bytes.
fn memfd_sized(len: usize) -> OwnedFd {
    let fd = memfd_create("casement-shm", MemfdFlags::CLOEXEC).expect("shm_buffer: memfd_create");
    ftruncate(&fd, len as u64).expect("shm_buffer: size the memfd");
    fd
}

impl ShmBuffer {
    /// The buffer's size in pixels.
    pub fn size(&self) -> PhysicalSize {
        self.size
    }

    /// The `wl_buffer`, to attach.
    pub fn buffer(&self) -> &WlBuffer {
        &self.buffer
    }

    /// Whether the compositor has sent `release` since the last [`ShmBuffer::fill`].
    pub fn is_released(&self) -> bool {
        self.released.load(Ordering::SeqCst)
    }

    /// Overwrites the pixels with `pattern`; the compositor sees them at the next commit that
    /// attaches this buffer.
    ///
    /// # Panics
    ///
    /// When the memfd cannot be written.
    pub fn fill(&mut self, pattern: Pattern) {
        self.file
            .write_all_at(&pattern.bytes(self.size), 0)
            .expect("ShmBuffer::fill: write the pattern into the memfd");
        self.released.store(false, Ordering::SeqCst);
    }
}

impl Drop for ShmBuffer {
    fn drop(&mut self) {
        self.buffer.destroy();
    }
}

impl Dispatch<WlShm, Plain> for ClientState {
    fn event(
        _: &mut Self,
        _: &WlShm,
        _: wl_shm::Event,
        _: &Plain,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WlShmPool, Plain> for ClientState {
    fn event(
        _: &mut Self,
        _: &WlShmPool,
        _: wl_shm_pool::Event,
        _: &Plain,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WlBuffer, Plain> for ClientState {
    fn event(
        _: &mut Self,
        _: &WlBuffer,
        event: wl_buffer::Event,
        plain: &Plain,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if matches!(event, wl_buffer::Event::Release) {
            plain.0.store(true, Ordering::SeqCst);
        }
    }
}
