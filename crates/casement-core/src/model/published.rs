//! What protocol clients are shown of the model, and when it changes (P1-M2).
//!
//! The window-list and workspace protocols publish state the model changes in many places. One
//! snapshot before a step and one comparison after it ask for the publishing effects, so no
//! transition has to remember them: `PublishToplevel` for each window whose title, app id,
//! state, key flag or output changed (or that appeared), `PublishWorkspaces` when the Spaces
//! changed, `PersistSpaces` when what the Spaces file stores changed, and the desktop's notes
//! (`desktop_notes`) for `quire_desktop_v1`.

use super::Model;
use super::desktop_notes::DeskSnapshot;
use super::space_persist::stored_file;
use crate::effect::Effect;
use crate::ids::{OutputKey, WindowId};
use crate::space::{SpaceBook, SpaceFile};
use casement_protocols::vocab::{Emphasis, PlacementMark, WindowStateKind};
use porter_core::ClaimedId;
use std::collections::{BTreeMap, BTreeSet};

/// What one window shows a window-list client.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Listing {
    title: String,
    claimed: ClaimedId,
    state: WindowStateKind,
    key: bool,
    mark: PlacementMark,
    output: Option<OutputKey>,
}

/// The model's publishable state at one instant.
#[derive(Debug, Clone)]
pub(super) struct Published {
    spaces: SpaceBook,
    remembered: SpaceFile,
    plugged: BTreeSet<OutputKey>,
    windows: BTreeMap<WindowId, Listing>,
    desk: DeskSnapshot,
}

impl Published {
    /// The state of `model` now.
    pub(super) fn of(model: &Model) -> Published {
        Published {
            spaces: model.spaces.clone(),
            remembered: model.remembered.clone(),
            plugged: model.outputs.0.keys().cloned().collect(),
            windows: model
                .windows
                .values()
                .map(|window| {
                    let listing = Listing {
                        title: window.title.0.clone(),
                        claimed: window.claimed_id.clone(),
                        state: window.state.kind(),
                        key: window.emphasis == Emphasis::Key,
                        mark: window.state.mark(),
                        output: model.output_of(window),
                    };
                    (window.id, listing)
                })
                .collect(),
            desk: DeskSnapshot::of(model),
        }
    }

    /// The effects that bring clients from this state to `after`'s.
    pub(super) fn changes_to(&self, after: &Model) -> Vec<Effect> {
        let now = Published::of(after);
        let toplevels = now
            .windows
            .iter()
            .filter(|(id, listing)| self.windows.get(*id) != Some(*listing))
            .map(|(id, _)| Effect::PublishToplevel(*id));
        let spaces_changed = self.spaces != now.spaces;
        let workspaces = spaces_changed.then_some(Effect::PublishWorkspaces);
        let persist = (spaces_changed || self.remembered != now.remembered)
            .then(|| (self.stored(), now.stored()))
            .filter(|(was, is)| was != is)
            .map(|_| Effect::PersistSpaces);
        toplevels
            .chain(workspaces)
            .chain(persist)
            .chain(self.desk.changes_to(after))
            .collect()
    }

    /// The Spaces file this state stores.
    fn stored(&self) -> SpaceFile {
        stored_file(&self.spaces, &self.remembered, &self.plugged)
    }
}

impl Model {
    /// The minimise rectangles of windows that are gone are forgotten.
    pub(super) fn forget_gone_minimise_rects(&mut self) {
        let windows = &self.windows;
        self.minimise_rects
            .retain(|window, _| windows.contains_key(window));
    }
}
