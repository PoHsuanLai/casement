//! GPUs. P0: the soft pieces a fake multi-GPU topology is built from (SP4, A4): a pixman
//! renderer with its own `ContextId` and a dma-buf allocator over memfds. P1-Z adds the
//! render-node seam and its real adapter.

mod soft;

pub use soft::{CopyCounters, FixedUdmabuf, SoftPixman};
