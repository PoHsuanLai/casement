//! Shared-memory buffers over a memfd, written with `write_all_at` (no mmap: no `unsafe`).

use crate::conn::TestClient;
use crate::pattern::Pattern;
use casement_core::geometry::PhysicalSize;

/// A `wl_buffer` over a memfd pool, holding a pattern.
#[derive(Debug)]
pub struct ShmBuffer {
    #[expect(dead_code, reason = "P0-T-client: the buffer's size")]
    size: PhysicalSize,
}

/// A `size` ARGB8888 buffer of `client` holding `pattern`.
pub fn shm_buffer(client: &mut TestClient, size: PhysicalSize, pattern: Pattern) -> ShmBuffer {
    let _ = (client, size, pattern);
    todo!("P0-T-client: shm_buffer")
}
