//! What a fixture starts with.

use crate::gpu::GpuTopology;
use casement_backend::{DmabufMode, RenderCostModel, SoftRenderer, VirtualOutputSpec};
use casement::relax::AckPolicy;
use casement_core::settings::CasementSettings;
use std::time::Duration;

/// A fixture's outputs, renderer, GPUs, settings and frame cost.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureSpec {
    /// The outputs plugged at start (C27: the backend's own spec type).
    pub outputs: Vec<VirtualOutputSpec>,
    /// The renderer.
    pub renderer: SoftRenderer,
    /// The fake GPUs (bodies at P1).
    pub gpus: GpuTopology,
    /// The settings.
    pub settings: CasementSettings,
    /// What a frame costs.
    pub cost: RenderCostModel,
    /// Whether `zwp_linux_dmabuf_v1` is offered (a fake main device over `/dev/udmabuf`).
    pub dmabuf: DmabufMode,
    /// Whether a buffer attached before the first ack is an error (A37; wlcs needs it lenient).
    pub ack_policy: AckPolicy,
}

impl FixtureSpec {
    /// One 1920x1080 output at 60 Hz and scale 1, pixman, one soft GPU, the default settings,
    /// every frame costing 4 ms.
    pub fn desktop() -> FixtureSpec {
        FixtureSpec {
            outputs: vec![
                VirtualOutputSpec::parse("HEADLESS-1:1920x1080@60000:120:0,0")
                    .expect("a literal spec in the grammar"),
            ],
            renderer: SoftRenderer::Pixman,
            gpus: GpuTopology::single(),
            settings: CasementSettings::defaults(),
            cost: RenderCostModel::Fixed(Duration::from_millis(4)),
            dmabuf: DmabufMode::None,
            ack_policy: AckPolicy::Strict,
        }
    }

    /// The same, with this dma-buf mode.
    pub fn with_dmabuf(self, dmabuf: DmabufMode) -> FixtureSpec {
        FixtureSpec { dmabuf, ..self }
    }

    /// The same, with this ack policy.
    pub fn with_ack_policy(self, ack_policy: AckPolicy) -> FixtureSpec {
        FixtureSpec { ack_policy, ..self }
    }

    /// The same, with these outputs.
    pub fn with_outputs(self, specs: &[&str]) -> FixtureSpec {
        FixtureSpec {
            outputs: specs
                .iter()
                .map(|spec| {
                    VirtualOutputSpec::parse(spec)
                        .unwrap_or_else(|error| panic!("output spec {spec}: {error}"))
                })
                .collect(),
            ..self
        }
    }

    /// The same, with this renderer.
    pub fn with_renderer(self, renderer: SoftRenderer) -> FixtureSpec {
        FixtureSpec { renderer, ..self }
    }
}
