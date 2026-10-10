//! GPUs. P0: the soft pieces a fake multi-GPU topology is built from (SP4, A4): a pixman
//! renderer with its own `ContextId` and a dma-buf allocator over memfds. P1-Z adds the
//! detection seam [`RenderNodes`] (reading an injected [`SysRoot`], A6), its real adapter
//! (feature `tty`), the stack that holds the render plan, the import routes and the format
//! source for dma-buf feedback. The pure plan is `casement_core::gpu`.

mod feedback;
mod import;
mod manager;
mod render_nodes;
mod soft;

#[cfg(feature = "tty")]
mod render_nodes_real;

pub use feedback::{FormatSource, format_table};
pub use import::{ImportRoute, route_for, tag_node};
pub use manager::GpuStack;
pub use render_nodes::{GpuError, GpuHandle, RenderNodes, SysRoot};
pub use soft::{CopyCounters, FixedUdmabuf, SoftPixman};

#[cfg(feature = "tty")]
pub use render_nodes_real::SysRenderNodes;
