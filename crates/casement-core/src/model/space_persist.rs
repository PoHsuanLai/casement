//! The Spaces across a restart: what the model stores and how it adopts a stored file.

use super::Model;
use super::moved::Moved;
use crate::ids::OutputKey;
use crate::space::{Space, SpaceBook, SpaceContent, SpaceFile, StoredSpace};
use crate::window::SpacePlace;
use porter_core::SpaceId;
use std::collections::BTreeSet;

/// The file that stores `spaces`: its desktops and what each output shows, and with them the
/// Spaces of `remembered` that wait for an output not plugged yet (so a Space is not forgotten
/// because its display was away for a run).
pub(super) fn stored_file(
    spaces: &SpaceBook,
    remembered: &SpaceFile,
    plugged: &BTreeSet<OutputKey>,
) -> SpaceFile {
    let mut file = SpaceFile::of_book(spaces);
    let waiting = |stored: &StoredSpace| {
        spaces.get(&stored.id).is_none()
            && stored
                .output
                .as_ref()
                .is_none_or(|output| !plugged.contains(output))
    };
    file.spaces
        .extend(remembered.spaces.iter().filter(|s| waiting(s)).cloned());
    file.shown.extend(
        remembered
            .shown
            .iter()
            .filter(|shown| !plugged.contains(&shown.output))
            .cloned(),
    );
    file
}

impl Model {
    /// The Spaces file for the model as it is (`SpaceFile::of_book`, plus the Spaces that wait
    /// for an output).
    pub fn space_file(&self) -> SpaceFile {
        let plugged: BTreeSet<OutputKey> = self.outputs.0.keys().cloned().collect();
        stored_file(&self.spaces, &self.remembered, &plugged)
    }

    /// Adopt a stored file: each stored Space keeps its id and place on its output once that
    /// output is plugged (a Space of an output not plugged yet waits); ids the file does not
    /// name are minted as before. Idempotent.
    pub(super) fn restore_spaces(self, file: SpaceFile) -> Moved {
        let outputs: Vec<OutputKey> = self.outputs.0.keys().cloned().collect();
        let model = Model {
            remembered: file,
            ..self
        };
        let model = outputs
            .iter()
            .fold(model, |model, output| model.adopt_ids(output));
        let choices: Vec<SpaceId> = outputs
            .iter()
            .filter_map(|output| model.stored_choice(output))
            .collect();
        choices
            .into_iter()
            .fold(Moved::of(model), |moved, id| moved.then(|m| m.show(&id)))
    }

    /// The stored Spaces of `output` (and the stored Spaces with no home) take their ids on it:
    /// the desktops it got by default are renamed in row order, the rest are made. A stored
    /// Space whose id the model already has is left as it is.
    pub(super) fn adopt_ids(self, output: &OutputKey) -> Model {
        let mut taking: Vec<StoredSpace> = self
            .remembered
            .spaces
            .iter()
            .filter(|stored| stored.output.as_ref().is_none_or(|home| home == output))
            .filter(|stored| self.spaces.get(&stored.id).is_none())
            .cloned()
            .collect();
        if taking.is_empty() {
            return self;
        }
        taking.sort_by(|a, b| (a.position, &a.id).cmp(&(b.position, &b.id)));
        let named: BTreeSet<&SpaceId> = self.remembered.spaces.iter().map(|s| &s.id).collect();
        let fresh: Vec<SpaceId> = self
            .spaces
            .row(output)
            .into_iter()
            .filter(|space| matches!(space.content, SpaceContent::Desktop))
            .filter(|space| !named.contains(&space.id))
            .map(|space| space.id.clone())
            .collect();
        let mut fresh = fresh.into_iter();
        let model = taking
            .into_iter()
            .fold(self, |model, stored| match fresh.next() {
                Some(old) => model.renamed(&old, &stored),
                None => model.with_stored(output, &stored),
            });
        model.renumbered(output)
    }

    /// The Space `old` is now `stored` (its id and place); its windows follow.
    fn renamed(self, old: &SpaceId, stored: &StoredSpace) -> Model {
        let Some(space) = self.spaces.get(old).cloned() else {
            return self;
        };
        let was_shown = self
            .spaces
            .shown_on(&space.output)
            .is_some_and(|shown| shown.id == *old);
        let output = space.output.clone();
        let mut spaces = self.spaces.without_space(old).with_space(Space {
            id: stored.id.clone(),
            position: stored.position,
            ..space
        });
        if was_shown {
            spaces = spaces.with_shown(output, stored.id.clone());
        }
        let mut model = Model { spaces, ..self };
        for window in model.windows.values_mut() {
            if window.place == SpacePlace::On(old.clone()) {
                window.place = SpacePlace::On(stored.id.clone());
            }
        }
        model
    }

    /// A desktop made from `stored` on `output`.
    fn with_stored(self, output: &OutputKey, stored: &StoredSpace) -> Model {
        let spaces = self.spaces.with_space(Space {
            id: stored.id.clone(),
            output: output.clone(),
            content: SpaceContent::Desktop,
            position: stored.position,
        });
        Model { spaces, ..self }
    }

    /// The Space the file says `output` showed, when `output` still has it.
    pub(super) fn stored_choice(&self, output: &OutputKey) -> Option<SpaceId> {
        self.remembered
            .shown
            .iter()
            .find(|stored| stored.output == *output)
            .map(|stored| stored.space.clone())
            .filter(|id| self.spaces.get(id).is_some_and(|s| s.output == *output))
    }
}
