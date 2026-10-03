//! The model's side of the focus machine: feeding it, and carrying out what it answers.

use super::Model;
use super::moved::Moved;
use super::states::toplevel_states;
use crate::app::AppKey;
use crate::effect::{ConfigureSize, ConfigureSpec, Effect};
use crate::focus::{self, FocusIn, FocusOut};
use crate::geometry::LogicalSize;
use crate::ids::WindowId;
use casement_protocols::vocab::{Attention, Emphasis};

impl Model {
    /// Focus after `input`, with its answers carried out on the records.
    pub(super) fn feed(self, input: FocusIn) -> Moved {
        let (focus, answers) = focus::step(self.focus.clone(), input, &self.view());
        let model = Model { focus, ..self };
        answers.into_iter().fold(Moved::of(model), |moved, answer| {
            moved.then(|model| model.carry_out(answer))
        })
    }

    /// One answer of the focus machine. The keyboard is not here: it is derived per step from
    /// who holds it, because a lock or a layer can hold it over the key window.
    fn carry_out(self, answer: FocusOut) -> Moved {
        match answer {
            FocusOut::Keyboard(_) => Moved::of(self),
            FocusOut::ActiveApp(_) => self.settled_active(),
            FocusOut::Raise(windows) => self.raised(&windows),
            FocusOut::Route(route) => Moved::of(self).with(Effect::Route(route)),
            FocusOut::Emphasis(changes) => self.emphasised(changes),
            FocusOut::Attention(app, level) => self.attention(&app, level),
            FocusOut::Bell(app) => Moved::of(self).with(Effect::Bell { app }),
        }
    }

    /// The active app has the person's eye: it no longer asks for it.
    fn settled_active(self) -> Moved {
        match self.focus.active.clone() {
            Some(app) => self.attention(&app, Attention::None),
            None => Moved::of(self),
        }
    }

    /// `windows` (bottom first) go to the top of the stack, and to the end of their apps'
    /// own order.
    pub(super) fn raised(self, windows: &[WindowId]) -> Moved {
        let model = windows.iter().fold(self, |mut model, id| {
            let rank = model.top_rank();
            let Some(window) = model.windows.get_mut(id) else {
                return model;
            };
            let on_top = rank.0 == window.stack.0.saturating_add(1);
            if !on_top {
                window.stack = rank;
            }
            let app = window.app.clone();
            if let Some(record) = model.apps.get_mut(&app) {
                record.windows.retain(|held| held != id);
                record.windows.push(*id);
            }
            model
        });
        Moved::of(model).with(Effect::Raise(windows.to_vec()))
    }

    /// The windows' emphasis changed: each is told, and a window that gained or lost the key
    /// is sent a configure with its activation state.
    fn emphasised(self, changes: Vec<(WindowId, Emphasis)>) -> Moved {
        let keyed: Vec<WindowId> = changes
            .iter()
            .filter(|(id, new)| {
                let old = self.windows.get(id).map(|window| window.emphasis);
                (*new == Emphasis::Key) != (old == Some(Emphasis::Key))
            })
            .map(|(id, _)| *id)
            .collect();
        let windows = changes
            .iter()
            .fold(self.windows.clone(), |mut windows, (id, emphasis)| {
                if let Some(window) = windows.get_mut(id) {
                    window.emphasis = *emphasis;
                }
                windows
            });
        let model = Model { windows, ..self };
        let configures: Vec<Effect> = keyed
            .into_iter()
            .filter_map(|id| model.configure_effect(id))
            .collect();
        Moved::of(model)
            .with(Effect::Emphasis(changes))
            .with_all(configures)
    }

    /// `app` asks for attention at `level` (or stops). Nothing is said when it did not change.
    fn attention(self, app: &AppKey, level: Attention) -> Moved {
        let Some(record) = self.apps.get(app) else {
            return Moved::of(self);
        };
        if record.attention == level {
            return Moved::of(self);
        }
        let mut model = self;
        if let Some(record) = model.apps.get_mut(app) {
            record.attention = level;
        }
        Moved::of(model).with(Effect::Attention {
            app: app.clone(),
            level,
        })
    }

    /// The configure `window` is sent now: no size of its own to ask for (layout's relayout
    /// asks for that), the states its record and focus imply, and the room its output has.
    pub(super) fn configure_effect(&self, id: WindowId) -> Option<Effect> {
        let window = self.windows.get(&id)?;
        let bounds = self
            .output_of(window)
            .and_then(|output| self.usable_rect(&output))
            .map_or(LogicalSize { w: 0, h: 0 }, |usable| usable.size);
        Some(Effect::Configure {
            window: id,
            configure: ConfigureSpec {
                size: ConfigureSize::Keep,
                states: toplevel_states(window, &self.focus),
                bounds,
            },
        })
    }
}
