//! Requests from workspace clients (`ext_workspace_manager_v1`) on the Spaces.

use super::Model;
use super::moved::Moved;
use crate::cause::Cause;
use crate::ids::OutputKey;
use crate::input::SpaceStep;
use crate::space::{Space, SpaceContent, WorkspaceRequest};
use porter_core::SpaceId;

impl Model {
    /// A workspace client asked something of the Spaces: show, hide, create, remove or move
    /// one. `Effect::PersistSpaces` and `Effect::PublishWorkspaces` follow every change (the
    /// model asks for them whenever its Spaces change, whoever changed them). Only desktops
    /// can be removed or moved, and an output keeps at least one; a request that cannot be
    /// carried out changes nothing.
    pub(super) fn workspace_request(self, request: WorkspaceRequest) -> Moved {
        match request {
            WorkspaceRequest::Activate(id) => self.show(&id),
            WorkspaceRequest::Deactivate(id) => self.hidden(&id),
            WorkspaceRequest::Remove(id) => self.removed(&id),
            WorkspaceRequest::Create { output, name } => {
                let _ = name;
                self.created(&output)
            }
            WorkspaceRequest::Assign { space, output } => self.assigned(&space, &output),
        }
    }

    /// The output shows the next Space of its row, else the previous, in place of `id`.
    fn hidden(self, id: &SpaceId) -> Moved {
        let Some(space) = self.spaces.get(id).cloned() else {
            return Moved::of(self);
        };
        let shown = self
            .spaces
            .shown_on(&space.output)
            .is_some_and(|shown| shown.id == *id);
        if !shown {
            return Moved::of(self);
        }
        let last = self.spaces.row(&space.output).last().map(|s| s.id.clone()) == Some(id.clone());
        let step = match last {
            true => SpaceStep::Previous,
            false => SpaceStep::Next,
        };
        self.switch(&space.output, step)
    }

    /// A new desktop at the end of `output`'s row.
    fn created(self, output: &OutputKey) -> Moved {
        let room = self.outputs.0.contains_key(output) && self.has_room_for_desktop(output);
        match room {
            true => Moved::of(self.with_space(output, SpaceContent::Desktop).0),
            false => Moved::of(self),
        }
    }

    /// The desktop `id` goes; its windows move to the desktop to its left (else to its right).
    fn removed(self, id: &SpaceId) -> Moved {
        let Some(space) = self.spaces.get(id).cloned() else {
            return Moved::of(self);
        };
        let Some(heir) = self.desktop_beside(&space) else {
            return Moved::of(self);
        };
        let was_shown = self
            .spaces
            .shown_on(&space.output)
            .is_some_and(|shown| shown.id == *id);
        let outputs = vec![space.output.clone()];
        let model = self.without_space(id, &heir);
        let moved = Moved::of(model).then(|model| model.redrawn(Cause::SpaceSwitch, outputs));
        match was_shown {
            true => moved.then(|model| model.show(&heir)),
            false => moved,
        }
    }

    /// The desktop `id` moves to the end of `output`'s row.
    fn assigned(self, id: &SpaceId, output: &OutputKey) -> Moved {
        let Some(space) = self.spaces.get(id).cloned() else {
            return Moved::of(self);
        };
        if space.output == *output || !self.outputs.0.contains_key(output) {
            return Moved::of(self);
        }
        let Some(heir) = self.desktop_beside(&space) else {
            return Moved::of(self);
        };
        let from = space.output.clone();
        let was_shown = self
            .spaces
            .shown_on(&from)
            .is_some_and(|shown| shown.id == *id);
        let position = crate::space::SpaceIndex(
            u32::try_from(self.spaces.row(output).len()).unwrap_or(u32::MAX),
        );
        let mut model = self;
        model.spaces = model.spaces.with_space(Space {
            output: output.clone(),
            position,
            ..space
        });
        if was_shown {
            model.spaces = model.spaces.without_shown(&from);
        }
        let model = model.renumbered(&from).renumbered(output);
        let outputs = vec![from, output.clone()];
        let moved = Moved::of(model).then(|model| model.redrawn(Cause::OutputChange, outputs));
        match was_shown {
            true => moved.then(|model| model.show(&heir)),
            false => moved,
        }
    }

    /// The desktop that would take over from `space`: the nearest desktop to its left, else
    /// the first other one of its row. None when `space` is not a desktop or is the only one.
    fn desktop_beside(&self, space: &Space) -> Option<SpaceId> {
        if !matches!(space.content, SpaceContent::Desktop) {
            return None;
        }
        let row = self.spaces.row(&space.output);
        let others = row
            .iter()
            .filter(|other| other.id != space.id)
            .filter(|other| matches!(other.content, SpaceContent::Desktop));
        let left = others
            .clone()
            .rfind(|other| other.position < space.position);
        left.or_else(|| others.clone().next()).map(|s| s.id.clone())
    }
}
