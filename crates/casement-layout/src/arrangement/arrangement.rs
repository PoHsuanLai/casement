//! The whole arrangement.

use super::{
    BackdropTarget, DividerTarget, LayoutReport, OverviewSlot, SpaceTarget, SpaceThumb,
    WindowTarget,
};

/// Where every window, Space and overview slot belongs, and what layout could not honour.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Arrangement {
    /// Every shown window's target (minimised windows have none).
    pub windows: Vec<WindowTarget>,
    /// Every Space's origin in its output's strip.
    pub spaces: Vec<SpaceTarget>,
    /// The overview's slots (empty outside an overview).
    pub overview: Vec<OverviewSlot>,
    /// Mission Control's Space row (empty outside it).
    pub space_thumbs: Vec<SpaceThumb>,
    /// Every Split View Space's divider strip (the compositor routes the pointer on it).
    pub dividers: Vec<DividerTarget>,
    /// Split View Spaces that draw a backdrop (a window centred in its half).
    pub backdrops: Vec<BackdropTarget>,
    /// What could not be honoured ("min sizes honoured or reported").
    pub reports: Vec<LayoutReport>,
}
