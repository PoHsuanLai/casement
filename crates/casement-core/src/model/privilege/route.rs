//! The model's side of the consent machine: which events reach it, what it is told about the
//! lock, and how its timer joins the model's.

use super::step::{Ctx, Input};
use super::{Class, Privilege};
use crate::effect::Effect;
use crate::lock::LockState;
use crate::model::Model;
use crate::model::moved::Moved;
use crate::policy::ConnectionRole;
use crate::time::TimerKey;

impl Model {
    /// The consent machine.
    pub fn privilege(&self) -> &Privilege {
        &self.privilege
    }

    /// `input` through the consent machine; its effects follow, and its deadline timer joins
    /// the model's timers.
    pub(in crate::model) fn privileged(self, input: Input) -> Moved {
        let ctx = Ctx {
            now: self.now,
            covered: !matches!(self.lock, LockState::Unlocked),
        };
        let stepped = self.privilege.step(input, ctx);
        let timers =
            stepped
                .effects
                .iter()
                .fold(self.timers.clone(), |book, effect| match effect {
                    Effect::ArmTimer(due) if due.key == TimerKey::AskDeadline => {
                        book.arm(due.clone())
                    }
                    Effect::CancelTimer(key) if *key == TimerKey::AskDeadline => book.cancel(key),
                    _ => book,
                });
        let model = Model {
            privilege: stepped.privilege,
            timers,
            ..self
        };
        Moved::of(model).with_all(stepped.effects)
    }

    /// A client connected through a socket of `role`.
    pub(in crate::model) fn privilege_connected(
        self,
        client: crate::ids::ClientKey,
        role: &ConnectionRole,
    ) -> Moved {
        self.privileged(Input::Connected {
            client,
            class: Class::of(role),
        })
    }
}
