//! A fake GPU topology: what detection would report, fed to `core::gpu::plan` exactly as the
//! real facts would be (P1).

use casement_backend::gpu::{FormatSource, GpuError};
use casement_backend::{FixtureName, GpuId};
use casement_core::CoreError;
use casement_core::OutputName;
use casement_core::gpu::{BootVga, ByPathId, DrmNodeId, FormatEntry, Fourcc, GpuFacts, Modifier};

/// The DRM major, and the first render-node minor: GPU `id` is node `226:(128 + id)` (A3).
const DRM_MAJOR: u32 = 226;
const FIRST_RENDER_MINOR: u32 = 128;

/// `AR24` and `XR24`, the fourcc codes of ARGB8888 and XRGB8888.
/// ARGB8888 as a DRM fourcc code.
pub const ARGB8888: u32 = 0x3432_5241;
/// XRGB8888 as a DRM fourcc code.
pub const XRGB8888: u32 = 0x3432_5258;

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

    /// Two GPUs, the display GPU A (AMD-like, the firmware's, driving `eDP-1`, importing XRGB8888
    /// only) and B (NVIDIA-like, driving `HDMI-A-1`, importing ARGB8888 and XRGB8888), both
    /// pixman, taking each other's buffers as `cross` says: "display on A, client buffers
    /// from B". A renders; B's tranche holds the one format A does not, ARGB8888.
    pub fn pair(cross: CrossImport) -> Result<GpuTopology, CoreError> {
        let panel = OutputName::parse("eDP-1")?;
        let hdmi = OutputName::parse("HDMI-A-1")?;
        Ok(GpuTopology {
            gpus: vec![
                SoftGpu::new(
                    GpuId(0),
                    SoftEngine::Pixman,
                    FormatSet::LinearCodes(vec![XRGB8888]),
                    cross,
                    VendorFact::Amd,
                )
                .driving(vec![panel]),
                SoftGpu::new(
                    GpuId(1),
                    SoftEngine::Pixman,
                    FormatSet::LinearOnly,
                    cross,
                    VendorFact::Nvidia,
                )
                .driving(vec![hdmi]),
            ],
        })
    }

    /// What detection reports for this topology: the facts `SysRenderNodes` reads back from the
    /// tree `FabricatedSysRoot` writes, in node order. The first GPU is the firmware's.
    pub fn facts(&self) -> Result<Vec<GpuFacts>, CoreError> {
        self.gpus
            .iter()
            .enumerate()
            .map(|(index, gpu)| {
                let boot = if index == 0 {
                    BootVga::Primary
                } else {
                    BootVga::Secondary
                };
                Ok::<GpuFacts, CoreError>(GpuFacts::new(
                    gpu.node(),
                    gpu.vendor,
                    boot,
                    gpu.drives.clone(),
                    gpu.by_path()?,
                ))
            })
            .collect()
    }
}

impl FormatSource for GpuTopology {
    /// The formats the soft GPU on `node` imports. A recorded table is not loadable here
    /// (`GpuError::NoFormats`): the fixture loader that records them does not exist yet.
    fn formats_of(&mut self, node: &DrmNodeId) -> Result<Vec<FormatEntry>, GpuError> {
        let gpu = self
            .gpus
            .iter()
            .find(|gpu| gpu.node() == *node)
            .ok_or(GpuError::UnknownNode(*node))?;
        match &gpu.formats {
            FormatSet::LinearOnly => Ok([ARGB8888, XRGB8888]
                .into_iter()
                .map(|format| FormatEntry {
                    format: Fourcc(format),
                    modifier: Modifier(0),
                })
                .collect()),
            FormatSet::LinearCodes(codes) => Ok(codes
                .iter()
                .map(|format| FormatEntry {
                    format: Fourcc(*format),
                    modifier: Modifier(0),
                })
                .collect()),
            FormatSet::Recorded(_) => Err(GpuError::NoFormats(*node)),
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

    /// This GPU's render node, `226:(128 + id)`.
    pub fn node(&self) -> DrmNodeId {
        DrmNodeId::new(DRM_MAJOR, FIRST_RENDER_MINOR + self.id.0)
    }

    /// This GPU's fabricated PCI address, `0000:<id + 1>:00.0`.
    pub fn by_path(&self) -> Result<ByPathId, CoreError> {
        ByPathId::parse(&format!("0000:{:02x}:00.0", self.id.0 + 1))
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
    /// Linear, with exactly these fourcc codes (so two GPUs can differ, which is what makes a
    /// second GPU's feedback tranche non-empty).
    LinearCodes(Vec<u32>),
    /// A recorded format table.
    Recorded(FixtureName),
}
