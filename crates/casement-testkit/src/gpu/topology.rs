//! A fake GPU topology: what detection would report, fed to `core::gpu::plan` exactly as the
//! real facts would be (P1).

use casement_backend::{FixtureName, GpuId};
use casement_core::OutputName;

/// The vendor a soft GPU claims: core's own type, so a topology feeds `casement_core::gpu::
/// plan_for` exactly as detection would.
pub use casement_core::gpu::VendorFact;

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
            gpus: vec![SoftGpu::new(
                GpuId(0),
                SoftEngine::Pixman,
                FormatSet::LinearOnly,
                CrossImport::Direct,
                VendorFact::Amd,
            )],
        }
    }
}

/// One soft GPU.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
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
    /// The connectors this GPU drives (what `drives` detection would report); none for a GPU
    /// that only renders.
    pub drives: Vec<OutputName>,
}

impl SoftGpu {
    /// A soft GPU that drives no connector.
    pub fn new(
        id: GpuId,
        engine: SoftEngine,
        formats: FormatSet,
        cross: CrossImport,
        vendor: VendorFact,
    ) -> SoftGpu {
        SoftGpu {
            id,
            engine,
            formats,
            cross,
            vendor,
            drives: Vec::new(),
        }
    }

    /// The same, driving these connectors.
    #[must_use]
    pub fn driving(self, drives: Vec<OutputName>) -> SoftGpu {
        SoftGpu { drives, ..self }
    }
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
