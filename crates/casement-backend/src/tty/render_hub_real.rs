//! The render GPU's renderer (feature `tty`): one GLES renderer on the GPU the render plan
//! names, shared by every display device that is not scanned out by that GPU. A thin adapter
//! (B7). Detection and the plan are pure and already made (`casement_core::gpu`); this holds
//! the one live piece, so every output's elements (blur, rounded clips, shaders, the closing
//! window's GPU copy) are built by a `GlesRenderer`, whichever GPU scans the output out.
//!
//! The GPU that scans an output out and also renders draws it directly (`Direct`); any other is
//! drawn here and copied across (`Copy`, `bridge_real.rs`). smithay's `MultiRenderer` is not used:
//! it is not a `GlesRenderer`, and the seam (`FrameSource::gles`) hands out elements for one.

use super::kms::KmsError;
use crate::gpu::SysRoot;
use casement_core::gpu::{DrmNodeId, OutputRoute, RenderPlan};
use smithay::backend::drm::DrmDeviceFd;
use smithay::backend::renderer::gles::GlesRenderer;
use smithay::reexports::gbm::Device as GbmDevice;
use std::cell::RefCell;
use std::fs::OpenOptions;
use std::io;
use std::rc::Rc;

/// A renderer other devices and the capture path lend.
pub(super) type SharedRenderer = Rc<RefCell<GlesRenderer>>;

/// The plan and the render GPU's renderer, made when first asked for.
#[derive(Debug)]
pub struct RenderHub {
    root: SysRoot,
    plan: RenderPlan,
    renderer: RefCell<Option<SharedRenderer>>,
}

impl RenderHub {
    /// A hub for `plan`, opening render nodes under `root`'s `/dev`.
    pub fn new(root: SysRoot, plan: RenderPlan) -> Rc<RenderHub> {
        Rc::new(RenderHub {
            root,
            plan,
            renderer: RefCell::new(None),
        })
    }

    /// The route of frames scanned out by the device whose render node is `dev` (a Linux
    /// `dev_t`); none for a device the plan scans no output out on.
    pub(super) fn route_of_device(&self, dev: u64) -> Option<OutputRoute> {
        self.plan.route_for_gpu(&DrmNodeId::from_dev_t(dev))
    }

    /// The render GPU's renderer if it exists already.
    pub(super) fn existing(&self) -> Option<SharedRenderer> {
        self.renderer.borrow().clone()
    }

    /// The render GPU's renderer, made on first use: on `via` when a display device of the
    /// render GPU offers its GBM device (one EGL display per native display), else on the
    /// render node itself.
    pub(super) fn renderer(
        &self,
        via: Option<&GbmDevice<DrmDeviceFd>>,
    ) -> Result<SharedRenderer, KmsError> {
        if let Some(renderer) = self.existing() {
            return Ok(renderer);
        }
        let made = match via {
            Some(gbm) => casement_render::gles_on(gbm.clone()),
            None => casement_render::gles_on(self.render_node_device()?),
        }
        .map_err(|error| KmsError::Io(io::Error::other(error.to_string())))?;
        let shared = Rc::new(RefCell::new(made));
        *self.renderer.borrow_mut() = Some(Rc::clone(&shared));
        Ok(shared)
    }

    /// The render node, opened for rendering (no DRM master, no session needed).
    fn render_node_device(&self) -> Result<GbmDevice<std::fs::File>, KmsError> {
        let path = self
            .root
            .dev()
            .join("dri")
            .join(format!("renderD{}", self.plan.render.minor));
        let file = OpenOptions::new().read(true).write(true).open(&path)?;
        Ok(GbmDevice::new(file)?)
    }
}
