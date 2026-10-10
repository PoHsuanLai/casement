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

impl Space {
    /// The name a person sees for it: "Desktop 1" for the first desktop of a row, and the kind
    /// for the others (ext-workspace's name; also what the account service is registered with).
    pub fn plain_name(&self) -> String {
        match self.content.kind() {
            SpaceKind::Desktop => format!("Desktop {}", self.position.0.saturating_add(1)),
            SpaceKind::FullScreen => "Full Screen".to_owned(),
            SpaceKind::SplitView => "Split View".to_owned(),
            _ => "Agent".to_owned(),
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::OutputName;
    use crate::output::{EdidFacts, EdidSerial};

    fn space(content: SpaceContent, position: u32) -> Space {
        let edid = EdidFacts {
            make: String::new(),
            model: String::new(),
            serial: EdidSerial::Unknown,
        };
        Space {
            id: SpaceId::parse("s-a").expect("an id"),
            output: OutputKey::of(&edid, &OutputName::parse("HDMI-A-1").expect("name")),
            content,
            position: SpaceIndex(position),
        }
    }

    #[test]
    fn a_desktop_is_named_by_its_place_and_the_others_by_their_kind() {
        let cases = [
            (SpaceContent::Desktop, 0, "Desktop 1"),
            (SpaceContent::Desktop, 1, "Desktop 2"),
            (SpaceContent::Agent(TaskToken("t".to_owned())), 3, "Agent"),
        ];
        for (content, position, want) in cases {
            assert_eq!(space(content, position).plain_name(), want);
        }
    }
}
