//! One Space's part of the scene.

use super::WindowSpec;
use casement_core::WindowId;
use casement_core::space::SplitPair;
use porter_core::SpaceId;

/// One Space and its windows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpaceSpec {
    /// The Space.
    pub space: SpaceId,
    /// How it is arranged.
    pub kind: SpaceArrangement,
    /// Its windows, bottom of the stack first.
    pub windows: Vec<WindowSpec>,
}

/// How a Space is arranged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpaceArrangement {
    /// A desktop.
    Desktop,
    /// One full-screen window.
    FullScreen(WindowId),
    /// Split View.
    Split(SplitPair),
}
