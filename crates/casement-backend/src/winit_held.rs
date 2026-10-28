//! The keys and buttons held down in the nested window, so that losing the host's focus can
//! release them: the host keeps the key-up that would have come, and a key stuck down in a
//! client repeats forever.

use casement_core::input::{ButtonCode, InputKind, KeyCode};
use casement_protocols::vocab::PressEdge;
use std::collections::BTreeSet;

/// What is held.
#[derive(Debug, Default)]
pub struct HeldInput {
    keys: BTreeSet<KeyCode>,
    buttons: BTreeSet<ButtonCode>,
}

impl HeldInput {
    /// Nothing held.
    pub fn new() -> HeldInput {
        HeldInput::default()
    }

    /// Takes `kind` into account: a press holds, a release lets go; anything else is not about
    /// holding.
    pub fn note(&mut self, kind: &InputKind) {
        match kind {
            InputKind::Key { code, edge } => Self::track(&mut self.keys, *code, *edge),
            InputKind::Button { button, edge } => Self::track(&mut self.buttons, *button, *edge),
            _ => {}
        }
    }

    fn track<T: Ord>(held: &mut BTreeSet<T>, code: T, edge: PressEdge) {
        match edge {
            PressEdge::Press => held.insert(code),
            PressEdge::Release => held.remove(&code),
        };
    }

    /// The releases of everything held, keys first; nothing is held after.
    pub fn release_all(&mut self) -> Vec<InputKind> {
        let keys = std::mem::take(&mut self.keys)
            .into_iter()
            .map(|code| InputKind::Key {
                code,
                edge: PressEdge::Release,
            });
        let buttons =
            std::mem::take(&mut self.buttons)
                .into_iter()
                .map(|button| InputKind::Button {
                    button,
                    edge: PressEdge::Release,
                });
        keys.chain(buttons).collect()
    }
}
