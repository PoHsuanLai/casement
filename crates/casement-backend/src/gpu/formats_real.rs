//! The formats each render node can sample (feature `tty`): `FormatSource` over a GLES renderer
//! on the node's own GBM device, a thin adapter (B7). A client that allocates a buffer in one of
//! these formats and modifiers can have it imported by the render GPU, which is what the
//! dma-buf feedback offers. The renderer is built once per node at start and dropped.

use super::{FormatSource, GpuError, SysRoot};
use casement_core::gpu::{DrmNodeId, FormatEntry, Fourcc, Modifier};
use smithay::backend::renderer::ImportDma;
use smithay::reexports::gbm::Device as GbmDevice;
use std::fs::OpenOptions;

/// The render nodes' formats, read from the node files under an injected root.
#[derive(Debug, Clone)]
pub struct RealFormats {
    root: SysRoot,
}

impl RealFormats {
    /// Formats of the nodes under `root`'s `/dev`.
    pub fn new(root: SysRoot) -> RealFormats {
        RealFormats { root }
    }
}

impl FormatSource for RealFormats {
    fn formats_of(&mut self, node: &DrmNodeId) -> Result<Vec<FormatEntry>, GpuError> {
        let refused = |source: std::io::Error| GpuError::Open {
            node: *node,
            source,
        };
        let path = self
            .root
            .dev()
            .join("dri")
            .join(format!("renderD{}", node.minor));
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .map_err(refused)?;
        let gbm = GbmDevice::new(file).map_err(refused)?;
        let renderer = casement_render::gles_on(gbm)
            .map_err(|error| refused(std::io::Error::other(error.to_string())))?;
        Ok(renderer
            .dmabuf_formats()
            .iter()
            .map(|format| FormatEntry {
                format: Fourcc(format.code as u32),
                modifier: Modifier(u64::from(format.modifier)),
            })
            .collect())
    }
}
