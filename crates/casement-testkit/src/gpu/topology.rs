//! A fake GPU topology: what detection would report, fed to `core::gpu::plan` exactly as the
//! real facts would be (P1).

use casement_backend::{FixtureName, GpuId};

/// The soft GPUs of a fixture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuTopology {
    /// The GPUs, the display GPU first.
    pub gpus: Vec<SoftGpu>,
}

impl GpuTopology {
    /// One pixman GPU (an AMD-like display GPU), linear formats, direct imports.
    pub fn single() -> GpuTopology {
        GpuTopology {
            gpus: vec![SoftGpu {
                id: GpuId(0),
                engine: SoftEngine::Pixman,
                formats: FormatSet::LinearOnly,
                cross: CrossImport::Direct,
                vendor: VendorFact::Amd,
            }],
        }
    }
}

/// One soft GPU.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoftGpu {
    /// Its number (its fabricated node is `226:(128 + id)`).
    pub id: GpuId,
    /// What renders on it.
    pub engine: SoftEngine,
    /// The formats it imports.
    pub formats: FormatSet,
    /// How it takes another GPU's buffers.
    pub cross: CrossImport,
    /// The vendor detection would report.
    pub vendor: VendorFact,
}

/// What renders on a soft GPU.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SoftEngine {
    /// pixman (exact; the multi-GPU tests' engine).
    Pixman,
    /// llvmpipe (effects; instances share one driver, so never two in a topology).
    Llvmpipe,
}

/// How a soft GPU takes another's buffers (smithay's `can_do_cross_device_imports`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CrossImport {
    /// Directly: smithay's dma shadow copy.
    Direct,
    /// Through a CPU copy (`ExportMem`, `ImportMem`).
    CopyOnly,
    /// Not at all (CopyOnly plus no cross-device exports from the source).
    Refuse,
}

/// The formats a soft GPU imports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormatSet {
    /// Linear ARGB8888 and XRGB8888 only.
    LinearOnly,
    /// A recorded format table.
    Recorded(FixtureName),
}

/// The vendor a soft GPU claims.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VendorFact {
    /// AMD.
    Amd,
    /// NVIDIA.
    Nvidia,
    /// Intel.
    Intel,
    /// Another PCI vendor id.
    Other(u16),
}
