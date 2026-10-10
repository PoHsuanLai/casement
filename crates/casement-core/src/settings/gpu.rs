//! GPU settings (`gpu.*`): which GPU renders (P1-T). The stored key and its conversion are in
//! `casement/src/config/`.

use crate::gpu::GpuChoice;

/// `gpu.*`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct GpuSettings {
    /// `gpu.render`: the GPU that renders every frame.
    pub render: GpuChoice,
}

impl GpuSettings {
    /// Settings that render on `render`.
    pub fn new(render: GpuChoice) -> GpuSettings {
        GpuSettings { render }
    }

    /// The default: the GPU that drives the outputs renders.
    ///
    /// ```
    /// use casement_core::gpu::GpuChoice;
    /// use casement_core::settings::GpuSettings;
    ///
    /// assert_eq!(GpuSettings::defaults().render, GpuChoice::Auto);
    /// ```
    pub fn defaults() -> GpuSettings {
        GpuSettings {
            render: GpuChoice::Auto,
        }
    }
}
