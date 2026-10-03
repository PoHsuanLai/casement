//! The soft GPU topology (11 §2.7, SP4 route 1, A3, A4): smithay's `GpuManager` over soft
//! devices found on the jail's fabricated `/sys/dev/char/226:(128+n)` nodes, each a
//! `casement_backend::gpu::SoftPixman` with its own context, buffers from
//! `casement_backend::gpu::FixedUdmabuf`. Bodies at P1 (P1-D4).

mod fabricated;
mod topology;

pub use fabricated::{FabricatedNode, fabricated_nodes};
pub use topology::{CrossImport, FormatSet, GpuTopology, SoftEngine, SoftGpu, VendorFact};
