//! The soft `GraphicsApi` (SP4 route 1, P1-Z; filled by P1-D4): smithay's `GpuManager` runs its
//! real import and copy code over soft devices found on the fabricated render nodes, each a
//! `casement_backend::gpu::SoftPixman` with its own context and buffers from `FixedUdmabuf`.
//! `GpuTopology::cross` maps onto `can_do_cross_device_imports`; unplugging a GPU is a "gone"
//! set the enumeration honours.

use super::{CrossImport, GpuTopology, SoftEngine, SoftGpu};
use casement_backend::gpu::{CounterHandle, FixedUdmabuf, SoftPixman};
use casement_backend::{BackendError, GpuId};
use smithay::backend::allocator::Allocator;
use smithay::backend::allocator::dmabuf::{AnyError, Dmabuf, DmabufAllocator};
use smithay::backend::drm::{CreateDrmNodeError, DrmNode};
use smithay::backend::renderer::multigpu::{ApiDevice, GraphicsApi};
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

    /// GPU `id` is unplugged: the next enumeration drops it. Through a `GpuManager` that is
    /// `manager.as_mut().unplug(id)`.
    pub fn unplug(&mut self, id: GpuId) {
        self.gone.insert(id.0);
    }
}

/// Why a soft device could not be made.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SoftApiError {
    /// The GPU's node is not a DRM node smithay can type.
    #[error("GPU {id:?}: {source}")]
    Node {
        /// The GPU.
        id: GpuId,
        /// Why `DrmNode::from_dev_id` refused it.
        #[source]
        source: CreateDrmNodeError,
    },
    /// The renderer or the allocator could not be made.
    #[error("soft GPU: {0}")]
    Device(#[from] BackendError),
    /// llvmpipe cannot stand in for a second GPU (instances share one driver).
    #[error("GPU {0:?}: only the pixman engine can be one of several GPUs")]
    Engine(GpuId),
}

/// One soft GPU smithay's `GpuManager` holds: a [`SoftPixman`] with its own context and a
/// [`FixedUdmabuf`] allocator.
#[derive(Debug)]
pub struct SoftDevice {
    id: GpuId,
    node: DrmNode,
    renderer: SoftPixman,
    allocator: DmabufAllocator<FixedUdmabuf>,
    cross: CrossImport,
}

impl SoftDevice {
    fn new(gpu: &SoftGpu, node: DrmNode) -> Result<SoftDevice, SoftApiError> {
        if gpu.engine != SoftEngine::Pixman {
            return Err(SoftApiError::Engine(gpu.id));
        }
        Ok(SoftDevice {
            id: gpu.id,
            node,
            renderer: SoftPixman::new()?,
            allocator: DmabufAllocator(FixedUdmabuf::new()?),
            cross: gpu.cross,
        })
    }

    /// Which topology GPU this is.
    pub fn id(&self) -> GpuId {
        self.id
    }

    /// A handle on this GPU's copy-path counters (A5: tests assert which path ran).
    pub fn counters(&self) -> CounterHandle {
        self.renderer.counter_handle()
    }
}

impl ApiDevice for SoftDevice {
    type Renderer = SoftPixman;

    fn renderer(&self) -> &SoftPixman {
        &self.renderer
    }

    fn renderer_mut(&mut self) -> &mut SoftPixman {
        &mut self.renderer
    }

    fn allocator(&mut self) -> &mut dyn Allocator<Buffer = Dmabuf, Error = AnyError> {
        &mut self.allocator
    }

    fn node(&self) -> &DrmNode {
        &self.node
    }

    fn can_do_cross_device_imports(&self) -> bool {
        self.cross == CrossImport::Direct
    }

    fn should_do_cross_device_exports(&self) -> bool {
        self.cross != CrossImport::Refuse
    }
}

impl GraphicsApi for SoftGraphicsApi {
    type Device = SoftDevice;
    type Error = SoftApiError;

    fn enumerate(&self, list: &mut Vec<SoftDevice>) -> Result<(), SoftApiError> {
        let mut present = Vec::new();
        for gpu in self
            .topology
            .gpus
            .iter()
            .filter(|gpu| !self.gone.contains(&gpu.id.0))
        {
            let node = gpu.node();
            let dev = rustix::fs::makedev(node.major, node.minor);
            let node = DrmNode::from_dev_id(dev)
                .map_err(|source| SoftApiError::Node { id: gpu.id, source })?;
            present.push((gpu, node));
        }
        list.retain(|device| present.iter().any(|(_, node)| *node == device.node));
        for (gpu, node) in present {
            if !list.iter().any(|device| device.node == node) {
                list.push(SoftDevice::new(gpu, node)?);
            }
        }
        Ok(())
    }

    fn needs_enumeration(&self) -> bool {
        !self.gone.is_empty()
    }

    fn identifier() -> &'static str {
        "soft"
    }
}
