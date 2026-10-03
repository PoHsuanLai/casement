//! One Space's record.

use crate::ids::{OutputKey, WindowId};
use casement_protocols::vocab::SpaceKind;
use ds_core::vocab::Fraction;
use porter_core::SpaceId;
use serde::{Deserialize, Serialize};

/// One Space: an output's desktop, or a full-screen, Split View or agent Space.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Space {
    /// Its stable id (also ext-workspace's id).
    pub id: SpaceId,
    /// The output it belongs to.
    pub output: OutputKey,
    /// What it holds.
    pub content: SpaceContent,
    /// Its place in the output's row.
    pub position: SpaceIndex,
}

/// What a Space holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum SpaceContent {
    /// A desktop: any number of windows.
    Desktop,
    /// One full-screen window.
    FullScreen(WindowId),
    /// Two windows side by side.
    SplitView(SplitPair),
    /// An agent's task Space.
    Agent(TaskToken),
}

impl SpaceContent {
    /// The content as the wire names it.
    pub fn kind(&self) -> SpaceKind {
        match self {
            SpaceContent::Desktop => SpaceKind::Desktop,
            SpaceContent::FullScreen(_) => SpaceKind::FullScreen,
            SpaceContent::SplitView(_) => SpaceKind::SplitView,
            SpaceContent::Agent(_) => SpaceKind::Agent,
        }
    }
}

/// A Space's place in its output's row, from 0 at the left.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SpaceIndex(pub u32);

/// The task an agent Space serves (an opaque token from the agent tier).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TaskToken(pub String);

/// The two windows of a Split View and where the divider sits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SplitPair {
    /// The left window.
    pub leading: WindowId,
    /// The right window.
    pub trailing: WindowId,
    /// The leading window's share of the width.
    pub divider: Fraction,
}
