//! How one output's picture gets from the GPU that draws it to the GPU that scans it out.

use super::{DrmNodeId, RenderPlan};
use crate::ids::OutputName;

/// The path of one output's frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum OutputRoute {
    /// The render GPU also scans this output out: no copy.
    Direct {
        /// The GPU that draws and scans out.
        gpu: DrmNodeId,
    },
    /// The render GPU draws, and the frame is copied to the GPU that scans the output out.
    Copy {
        /// The GPU that draws.
        render: DrmNodeId,
        /// The GPU that scans out.
        scanout: DrmNodeId,
    },
}

impl OutputRoute {
    /// The GPU whose renderer draws this output's frames.
    pub fn renders_on(&self) -> DrmNodeId {
        match *self {
            OutputRoute::Direct { gpu } => gpu,
            OutputRoute::Copy { render, .. } => render,
        }
    }

    /// The GPU that scans this output out.
    pub fn scans_out_on(&self) -> DrmNodeId {
        match *self {
            OutputRoute::Direct { gpu } => gpu,
            OutputRoute::Copy { scanout, .. } => scanout,
        }
    }
}

impl RenderPlan {
    /// The route of `output`'s frames; none for a connector the plan does not place.
    ///
    /// ```
    /// use casement_core::OutputName;
    /// use casement_core::gpu::{DrmNodeId, OutputRoute, RenderPlan};
    /// use std::collections::BTreeMap;
    ///
    /// let (render, other) = (DrmNodeId::new(226, 128), DrmNodeId::new(226, 129));
    /// let name = OutputName::parse("eDP-1").expect("a connector");
    /// let plan = RenderPlan::new(render, BTreeMap::from([(name.clone(), other)]));
    /// assert_eq!(
    ///     plan.route_of(&name),
    ///     Some(OutputRoute::Copy { render, scanout: other })
    /// );
    /// ```
    pub fn route_of(&self, output: &OutputName) -> Option<OutputRoute> {
        let scanout = *self.scanout.get(output)?;
        Some(if scanout == self.render {
            OutputRoute::Direct { gpu: scanout }
        } else {
            OutputRoute::Copy {
                render: self.render,
                scanout,
            }
        })
    }

    /// The route of every placed output, in connector order.
    pub fn routes(&self) -> Vec<(OutputName, OutputRoute)> {
        self.scanout
            .keys()
            .filter_map(|name| Some((name.clone(), self.route_of(name)?)))
            .collect()
    }
}
