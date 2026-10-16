//! Window records: mapping one onto a Space, unmapping it, and what its client says about it.

use super::Model;
use super::moved::Moved;
use crate::app::AppKey;
use crate::cause::Cause;
use crate::effect::{CoreNote, Effect};
use crate::event::{MappedToplevel, ToplevelMeta, WindowHintChange};
use crate::focus::{FocusIn, TokenCheck};
use crate::geometry::LogicalSize;
use crate::ids::WindowId;
use crate::settings::Enabled;
use crate::space::SpaceContent;
use crate::window::{Parent, SpacePlace, Window, WindowHints, WindowState, WindowTitle};
use casement_protocols::vocab::{Emphasis, Modality, SensitivityLevel, WindowRole};

impl Model {
    /// A toplevel mapped: it joins its app, lands on a Space, and focus decides whether it
    /// takes the key (a valid activation token) or its app asks for attention (A7).
    pub(super) fn map(self, mapped: MappedToplevel) -> Moved {
        if self.windows.contains_key(&mapped.window) {
            return Moved::of(self);
        }
        let id = mapped.window;
        let (place, home) = self.placement_of_new(&mapped);
        let parent = self.parent_of(&mapped.app, id, mapped.parent);
        let sheet_of = match (parent, mapped.modality) {
            (Parent::Window(held), Modality::WindowModal) => Some(held),
            _ => None,
        };
        let window = Window {
            id,
            client: mapped.client,
            app: mapped.app.clone(),
            origin: mapped.origin,
            role: mapped.role,
            modality: mapped.modality,
            place,
            home,
            stack: self.top_rank(),
            state: WindowState::New(mapped.first_size),
            parent,
            emphasis: Emphasis::Inactive,
            sensitivity: SensitivityLevel::Normal,
            limits: mapped.limits,
            title: WindowTitle::default(),
            claimed_id: mapped.claimed_id,
            hints: WindowHints::unset(),
        };
        let window_app = mapped.app.clone();
        let mut model = self.with_app_window(&mapped.app, id);
        model
            .clients
            .entry(mapped.client)
            .or_default()
            .insert(mapped.app);
        model.windows.insert(id, window);
        let outputs = model.outputs_of(&[id]);
        let token = mapped.activation;
        let follow = model.settings.spaces.switch_on_activate;
        Moved::of(model)
            .then(|model| match token {
                TokenCheck::Valid => model.unhide_app(&window_app),
                TokenCheck::Stale | TokenCheck::Missing => Moved::of(model),
            })
            .then(|model| match (token, follow, model.place_of(id)) {
                (TokenCheck::Valid, Enabled::On, Some(space)) => model.show(&space),
                _ => Moved::of(model),
            })
            .then(|model| model.listed(id))
            .then(|model| model.feed_mapped(id, token))
            .then(|model| match sheet_of {
                Some(parent) => model.feed(FocusIn::SheetOpened { parent, sheet: id }),
                None => Moved::of(model),
            })
            .configured_once(id)
            .then(|model| model.redrawn(Cause::Open, outputs))
    }

    /// Whether the person can see window `id` now.
    pub(super) fn sees_window(&self, id: WindowId) -> bool {
        self.windows
            .get(&id)
            .is_some_and(|window| self.is_seen(window))
    }

    /// The Space window `id` is on, if it is on one Space.
    fn place_of(&self, id: WindowId) -> Option<porter_core::SpaceId> {
        match self.windows.get(&id).map(|window| &window.place) {
            Some(SpacePlace::On(space)) => Some(space.clone()),
            Some(SpacePlace::AllSpaces) | None => None,
        }
    }

    /// Focus hears the window mapped: a valid token that the person can act on keys it,
    /// anything else lets focus decide whether its app only asks for attention (A7).
    fn feed_mapped(self, id: WindowId, token: TokenCheck) -> Moved {
        let seen = self.sees_window(id);
        match (token, seen) {
            (TokenCheck::Valid, true) => self.feed(FocusIn::Token {
                window: id,
                token: TokenCheck::Valid,
            }),
            _ => self.feed(FocusIn::Mapped(id)),
        }
    }

    /// `asked` as the parent of window `id` of `app`: a mapped window of the same app, not the
    /// window itself, else none (a sheet or a dialog belongs to its own app's window).
    fn parent_of(&self, app: &AppKey, id: WindowId, asked: Parent) -> Parent {
        match asked {
            Parent::Window(held)
                if held != id && self.windows.get(&held).is_some_and(|w| w.app == *app) =>
            {
                asked
            }
            Parent::Window(_) | Parent::None => Parent::None,
        }
    }

    /// Where a window that just mapped goes: with its parent; on every Space of the primary
    /// output for a shell window; else on the desktop the person is working on. With no output
    /// plugged it waits on every Space, at no home, until one is.
    fn placement_of_new(
        &self,
        mapped: &MappedToplevel,
    ) -> (SpacePlace, Option<crate::ids::OutputKey>) {
        let parent = match mapped.parent {
            Parent::Window(held) => self.windows.get(&held),
            Parent::None => None,
        };
        if let Some(parent) = parent {
            return (parent.place.clone(), parent.home.clone());
        }
        match (mapped.role, self.working_output()) {
            (WindowRole::Shell, output) => (SpacePlace::AllSpaces, output),
            (_, Some(output)) => match self.landing_space(&output) {
                Some(space) => (SpacePlace::On(space), None),
                None => (SpacePlace::AllSpaces, Some(output)),
            },
            (_, None) => (SpacePlace::AllSpaces, None),
        }
    }

    /// A toplevel unmapped or was destroyed.
    pub(super) fn unmap(self, id: WindowId) -> Moved {
        let Some(window) = self.windows.get(&id).cloned() else {
            return Moved::of(self).with(Effect::Note(CoreNote::UnknownWindow(id)));
        };
        let outputs = self.outputs_of(&[id]);
        let was_seen = window.role != WindowRole::Shell && self.sees_window(id);
        let held = self.spaces_naming(id);
        let mut model = self.without_app_window(&window.app, id);
        model.windows.remove(&id);
        for child in model.windows.values_mut() {
            if child.parent == Parent::Window(id) {
                child.parent = Parent::None;
            }
        }
        let closed_sheet = match (window.parent, window.modality) {
            (Parent::Window(parent), Modality::WindowModal) => Some(parent),
            _ => None,
        };
        let moved = held.into_iter().fold(Moved::of(model), |moved, space| {
            moved.then(|model| match space.content {
                SpaceContent::FullScreen(_) => model.retire_full_screen(&space.id),
                SpaceContent::SplitView(pair) => model.split_survivor(&space.id, pair, id),
                SpaceContent::Desktop | SpaceContent::Agent(_) => Moved::of(model),
            })
        });
        moved
            .then(|model| model.choice_over_for(&[id]))
            .then(|model| model.with_effect(Effect::UnlistWindow(id)))
            .then(|model| model.feed(FocusIn::Unmapped(id)))
            .then(|model| match closed_sheet {
                Some(parent) => model.feed(FocusIn::SheetClosed { parent }),
                None => Moved::of(model),
            })
            .then(|model| model.redrawn(Cause::Close, outputs.clone()))
            .then(move |model| match was_seen {
                true => model.leave_when_bare(&outputs),
                false => Moved::of(model),
            })
    }

    /// A toplevel committed a size: an unplaced or floating window keeps it.
    pub(super) fn commit(self, id: WindowId, size: LogicalSize) -> Moved {
        let Some(window) = self.windows.get(&id) else {
            return Moved::of(self).with(Effect::Note(CoreNote::UnknownWindow(id)));
        };
        let state = match &window.state {
            WindowState::New(_) => WindowState::New(size),
            WindowState::Floating(rect) => {
                WindowState::Floating(crate::geometry::LogicalRect::new(rect.origin, size))
            }
            other => other.clone(),
        };
        if state == window.state {
            return Moved::of(self);
        }
        let outputs = self.outputs_of(&[id]);
        self.with_state(id, state)
            .then(|model| model.redrawn(Cause::Configure, outputs))
    }

    /// A mapped toplevel's dialog hint changed. A window that now blocks its parent opens a
    /// sheet on it, one that no longer does closes it (as at the map).
    pub(super) fn modality(self, id: WindowId, modality: Modality) -> Moved {
        let Some(window) = self.windows.get(&id).cloned() else {
            return Moved::of(self).with(Effect::Note(CoreNote::UnknownWindow(id)));
        };
        if window.modality == modality {
            return Moved::of(self);
        }
        let sheet_of = |modality: Modality| match (window.parent, modality) {
            (Parent::Window(parent), Modality::WindowModal) => Some(parent),
            _ => None,
        };
        let (before, after) = (sheet_of(window.modality), sheet_of(modality));
        let mut model = self;
        set(&mut model, id, |held| held.modality = modality);
        Moved::of(model).then(|model| match (before, after) {
            (None, Some(parent)) => model.feed(FocusIn::SheetOpened { parent, sheet: id }),
            (Some(parent), None) => model.feed(FocusIn::SheetClosed { parent }),
            _ => Moved::of(model),
        })
    }

    /// A toplevel's metadata changed.
    pub(super) fn meta(self, id: WindowId, meta: ToplevelMeta) -> Moved {
        let Some(window) = self.windows.get(&id) else {
            return Moved::of(self).with(Effect::Note(CoreNote::UnknownWindow(id)));
        };
        let (limits_before, app) = (window.limits, window.app.clone());
        let outputs = self.outputs_of(&[id]);
        let mut model = self;
        let announces = matches!(meta, ToplevelMeta::Title(_) | ToplevelMeta::ClaimedId(_));
        let relayout = match meta {
            ToplevelMeta::Title(title) => {
                set(&mut model, id, |window| window.title = title);
                false
            }
            ToplevelMeta::ClaimedId(claimed) => {
                set(&mut model, id, |window| window.claimed_id = claimed);
                false
            }
            ToplevelMeta::Parent(parent) => {
                let held = model.parent_of(&app, id, parent);
                set(&mut model, id, |window| window.parent = held);
                false
            }
            ToplevelMeta::Limits(limits) => {
                let changed = limits_before != limits;
                set(&mut model, id, |window| window.limits = limits);
                changed
            }
        };
        let moved = match relayout {
            true => model.redrawn(Cause::Configure, outputs),
            false => Moved::of(model),
        };
        match announces {
            true => moved
                .then(|model| model.listed(id))
                .then(|model| model.pip_joined(id)),
            false => moved,
        }
    }

    /// A `quire_window_v1` hint changed.
    pub(super) fn hint(self, id: WindowId, hint: WindowHintChange) -> Moved {
        if !self.windows.contains_key(&id) {
            return Moved::of(self).with(Effect::Note(CoreNote::UnknownWindow(id)));
        }
        let mut model = self;
        set(&mut model, id, |window| match hint {
            WindowHintChange::FirstMouse(regions) => window.hints.first_mouse = regions,
            WindowHintChange::Frame(frame) => window.hints.frame = frame,
            WindowHintChange::Role(role) => window.role = role,
            WindowHintChange::A11y(root) => window.hints.a11y = root,
            WindowHintChange::Edited(edited) => window.hints.edited = edited,
            WindowHintChange::Menu(menu) => window.hints.menu = menu,
        });
        Moved::of(model).then(|model| model.pip_joined(id))
    }

    /// The window is listed on the foreign toplevel list, or its listing is brought up to date.
    pub(super) fn listed(self, id: WindowId) -> Moved {
        self.with_effect(Effect::ListWindow(id))
    }

    /// The model unchanged and `effect` asked.
    pub(super) fn with_effect(self, effect: Effect) -> Moved {
        Moved::of(self).with(effect)
    }

    /// The window `id` in state `state`.
    pub(super) fn with_state(self, id: WindowId, state: WindowState) -> Moved {
        let mut model = self;
        set(&mut model, id, |window| window.state = state);
        Moved::of(model)
    }

    /// The Space `id` holds `content`.
    pub(super) fn with_content(self, id: &porter_core::SpaceId, content: SpaceContent) -> Moved {
        let space = self.spaces.get(id).cloned();
        let spaces = match space {
            Some(space) => self
                .spaces
                .with_space(crate::space::Space { content, ..space }),
            None => self.spaces,
        };
        Moved::of(Model { spaces, ..self })
    }
}

/// `change` applied to the window `id`, when the model has it.
fn set(model: &mut Model, id: WindowId, change: impl FnOnce(&mut Window)) {
    if let Some(window) = model.windows.get_mut(&id) {
        change(window);
    }
}
