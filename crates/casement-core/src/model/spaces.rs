//! Spaces: a desktop for every output, switching between them, and the Spaces full-screen
//! windows own.

use super::Model;
use super::moved::Moved;
use crate::cause::Cause;
use crate::effect::Effect;
use crate::focus::FocusIn;
use crate::ids::{OutputKey, WindowId};
use crate::input::SpaceStep;
use crate::space::{Space, SpaceContent, SpaceIndex};
use crate::window::SpacePlace;
use porter_core::SpaceId;

impl Model {
    /// A new Space holding `content` at the end of `output`'s row, and its id.
    pub(super) fn with_space(self, output: &OutputKey, content: SpaceContent) -> (Model, SpaceId) {
        let (seed, id) = self.seed.clone().next();
        let position = SpaceIndex(u32::try_from(self.spaces.row(output).len()).unwrap_or(u32::MAX));
        let space = Space {
            id: id.clone(),
            output: output.clone(),
            content,
            position,
        };
        let model = Model {
            spaces: self.spaces.with_space(space),
            seed,
            ..self
        };
        (model, id)
    }

    /// `output`'s Spaces numbered 0, 1, 2 in row order.
    pub(super) fn renumbered(self, output: &OutputKey) -> Model {
        let row: Vec<Space> = self.spaces.row(output).into_iter().cloned().collect();
        let spaces = row
            .into_iter()
            .enumerate()
            .fold(self.spaces, |spaces, (index, space)| {
                let position = SpaceIndex(u32::try_from(index).unwrap_or(u32::MAX));
                spaces.with_space(Space { position, ..space })
            });
        Model { spaces, ..self }
    }

    /// `output` shows the Space `id`: the layout and the frames follow, and focus looks again
    /// at what the person can see. Nothing happens when it already shows it.
    pub(super) fn show(self, id: &SpaceId) -> Moved {
        let Some(space) = self.spaces.get(id).cloned() else {
            return Moved::of(self);
        };
        if self.spaces.shown_on(&space.output).map(|shown| &shown.id) == Some(id) {
            return Moved::of(self);
        }
        let output = space.output;
        let model = Model {
            spaces: self.spaces.with_shown(output.clone(), id.clone()),
            ..self
        };
        let input = FocusIn::SpaceShown {
            output: output.clone(),
            space: id.clone(),
        };
        let shown = id.clone();
        Moved::of(model)
            .with(Effect::Relayout(Cause::SpaceSwitch))
            .with(Effect::RequestFrames(output))
            .then(|model| model.feed(input))
            .then(move |model| model.choice_unless_on(&shown))
    }

    /// `output` shows the Space `step` names, when the row has one there.
    pub(super) fn switch(self, output: &OutputKey, step: SpaceStep) -> Moved {
        let row: Vec<SpaceId> = self
            .spaces
            .row(output)
            .into_iter()
            .map(|space| space.id.clone())
            .collect();
        let current = self
            .spaces
            .shown_on(output)
            .and_then(|shown| row.iter().position(|id| *id == shown.id));
        let target = match (step, current) {
            (SpaceStep::Previous, Some(at)) => at.checked_sub(1),
            (SpaceStep::Next, Some(at)) => at.checked_add(1),
            (SpaceStep::Number(n), _) => usize::from(n).checked_sub(1),
            (SpaceStep::Previous | SpaceStep::Next, None) => None,
        };
        match target.and_then(|at| row.get(at)) {
            Some(id) => self.show(id),
            None => Moved::of(self),
        }
    }

    /// The model without the Space `id`; its windows go to `heir`.
    pub(super) fn without_space(self, id: &SpaceId, heir: &SpaceId) -> Model {
        let output = self.spaces.get(id).map(|space| space.output.clone());
        let windows = self
            .windows
            .into_iter()
            .map(|(key, mut window)| {
                if window.place == SpacePlace::On(id.clone()) {
                    window.place = SpacePlace::On(heir.clone());
                }
                (key, window)
            })
            .collect();
        let model = Model {
            windows,
            spaces: self.spaces.without_space(id),
            ..self
        };
        match output {
            Some(output) => model.renumbered(&output),
            None => model,
        }
    }

    /// The full-screen Space `id` ends (its window left full screen or unmapped): its windows
    /// go to the desktop to its left, else the output's first desktop, and that desktop is
    /// shown when the Space was.
    pub(super) fn retire_full_screen(self, id: &SpaceId) -> Moved {
        let Some(space) = self.spaces.get(id).cloned() else {
            return Moved::of(self);
        };
        let was_shown = self.spaces.shown_on(&space.output).map(|shown| &shown.id) == Some(id);
        let (model, heir) = self.heir_of(&space);
        let heir_for_show = heir.clone();
        let model = model.without_space(id, &heir);
        match was_shown {
            true => Moved::of(model).then(|model| model.show(&heir_for_show)),
            false => Moved::of(model),
        }
    }

    /// The desktop that takes over from `space`: the nearest one to its left, else the first
    /// of its row, else a new one.
    fn heir_of(self, space: &Space) -> (Model, SpaceId) {
        let row = self.spaces.row(&space.output);
        let desktops = row
            .iter()
            .filter(|other| matches!(other.content, SpaceContent::Desktop));
        let left = desktops
            .clone()
            .rfind(|other| other.position < space.position);
        let found = left
            .or_else(|| desktops.clone().next())
            .map(|s| s.id.clone());
        match found {
            Some(id) => (self, id),
            None => self.with_space(&space.output, SpaceContent::Desktop),
        }
    }

    /// The Spaces whose content names `window` (its full-screen Space, a Split View it is in).
    pub(super) fn spaces_naming(&self, window: WindowId) -> Vec<Space> {
        self.spaces
            .iter()
            .filter(|space| match &space.content {
                SpaceContent::FullScreen(held) => *held == window,
                SpaceContent::SplitView(pair) => pair.leading == window || pair.trailing == window,
                SpaceContent::Desktop | SpaceContent::Agent(_) => false,
            })
            .cloned()
            .collect()
    }
}
