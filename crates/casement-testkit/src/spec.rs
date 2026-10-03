//! What a fixture starts with.

use crate::gpu::GpuTopology;
use casement_backend::{RenderCostModel, SoftRenderer, VirtualOutputSpec};
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
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_desktop_spec_is_one_plain_output() {
        let spec = FixtureSpec::desktop();
        assert_eq!(spec.outputs.len(), 1);
        assert_eq!(spec.outputs[0].text(), "HEADLESS-1:1920x1080@60000:120:0,0");
        assert_eq!(spec.renderer, SoftRenderer::Pixman);
        let three = spec.with_outputs(&[
            "HEADLESS-1:1920x1080@60000:120:0,0",
            "HEADLESS-2:2880x1620@60000:180:1920,0",
            "HEADLESS-3:3840x2160@60000:240:3840,0",
        ]);
        assert_eq!(three.outputs.len(), 3);
    }
}
