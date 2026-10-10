//! The soft `GraphicsApi` (SP4 route 1, P1-Z; filled by P1-D4): smithay's `GpuManager` runs its
//! real import and copy code over soft devices found on the fabricated render nodes, each a
//! `casement_backend::gpu::SoftPixman` with its own context and buffers from `FixedUdmabuf`.
//! `GpuTopology::cross` maps onto `can_do_cross_device_imports`; unplugging a GPU is a "gone"
//! set the enumeration honours.

use super::GpuTopology;
use casement_backend::GpuId;
use std::collections::BTreeSet;

/// The soft graphics API over a topology.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoftGraphicsApi {
    /// The GPUs it enumerates.
    pub topology: GpuTopology,
    /// The GPUs unplugged since (they enumerate no more, and their buffers import as
    /// `DeviceMissing`).
    pub gone: BTreeSet<u32>,
}

impl SoftGraphicsApi {
    /// An API over `topology` with every GPU present.
    pub fn new(topology: GpuTopology) -> SoftGraphicsApi {
        SoftGraphicsApi {
            topology,
            gone: BTreeSet::new(),
        }
    }

    /// GPU `id` is unplugged.
    pub fn unplug(&mut self, id: GpuId) {
        let _ = id;
        todo!("P1-D4: SoftGraphicsApi::unplug")
    }
}
