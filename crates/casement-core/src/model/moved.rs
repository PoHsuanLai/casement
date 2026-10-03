//! A transition's result: the next model and the effects asked on the way to it.

use super::Model;
use crate::effect::{ConfigureSpec, Effect};
use crate::ids::WindowId;

/// A model and the effects a handler asked for, in order.
#[derive(Debug)]
pub(super) struct Moved {
    pub(super) model: Model,
    pub(super) effects: Vec<Effect>,
}

impl Moved {
    /// `model` unchanged, no effects.
    pub(super) fn of(model: Model) -> Moved {
        Moved {
            model,
            effects: Vec::new(),
        }
    }

    /// The same with `effect` asked last.
    pub(super) fn with(self, effect: Effect) -> Moved {
        self.with_all([effect])
    }

    /// The same with `effects` asked last.
    pub(super) fn with_all(self, effects: impl IntoIterator<Item = Effect>) -> Moved {
        Moved {
            effects: self.effects.into_iter().chain(effects).collect(),
            ..self
        }
    }

    /// The same, then `next` on the model it reached; the effects of both, in order.
    pub(super) fn then(self, next: impl FnOnce(Model) -> Moved) -> Moved {
        let Moved { model, effects } = self;
        let after = next(model);
        Moved {
            model: after.model,
            effects: effects.into_iter().chain(after.effects).collect(),
        }
    }

    /// The configure this window was sent, if one is among the effects.
    pub(super) fn configure_of(&self, window: WindowId) -> Option<&ConfigureSpec> {
        self.effects.iter().find_map(|effect| match effect {
            Effect::Configure {
                window: sent,
                configure,
            } if *sent == window => Some(configure),
            _ => None,
        })
    }
}

impl Moved {
    /// The same, with a configure for `window` unless focus's answers already sent one.
    pub(super) fn configured_once(self, window: WindowId) -> Moved {
        if self.configure_of(window).is_some() {
            return self;
        }
        let configure = self.model.configure_effect(window);
        self.with_all(configure)
    }
}
