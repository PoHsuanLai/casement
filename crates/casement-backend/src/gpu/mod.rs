//! GPUs. P0: the soft pieces a fake multi-GPU topology is built from (SP4, A4): a pixman
//! renderer with its own `ContextId` and a dma-buf allocator over memfds. P1-Z adds the
//! detection seam [`RenderNodes`] (reading an injected [`SysRoot`], A6) and its adapter
//! (plain file reads, so it builds without the `tty` feature and the test kit runs it over a
//! fabricated tree), the stack that holds the render plan, the import routes and the format
//! source for dma-buf feedback. The pure plan is `casement_core::gpu`.

mod feedback;
mod import;
mod manager;
mod render_nodes;
mod render_nodes_real;
mod soft;

pub use feedback::{FormatSource, format_table};
pub use import::{CrossSupport, ImportRoute, route_for, route_with, tag_node};
pub use manager::GpuStack;
pub use render_nodes::{GpuError, GpuHandle, RenderNodes, SysRoot};
pub use render_nodes_real::SysRenderNodes;
pub use soft::{CopyCounters, CounterHandle, FixedUdmabuf, SoftFrame, SoftPixman};
