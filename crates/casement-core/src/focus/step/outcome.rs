//! A handler's answer, and the effects that follow from the move it made.

use super::super::{Focus, FocusOut, FocusView, PressRoute};
use crate::ids::WindowId;
use casement_protocols::vocab::Emphasis;
use std::collections::BTreeSet;

/// Where focus went and what the event asked for besides what the move implies.
pub(super) struct Outcome {
    /// The focus after the event.
    pub(super) focus: Focus,
    /// Windows to raise, bottom first.
    pub(super) raise: Vec<WindowId>,
    /// How the press goes, for a press.
    pub(super) route: Option<PressRoute>,
    /// Attention and the bell, which no move implies.
    pub(super) tail: Vec<FocusOut>,
}

impl Outcome {
    /// Focus is `focus` (moved or not) and nothing else was asked.
    pub(super) fn of(focus: Focus) -> Outcome {
        Outcome {
            focus,
            raise: Vec::new(),
            route: None,
            tail: Vec::new(),
        }
    }

    /// Focus did not move; `effects` are asked (attention, the bell).
    pub(super) fn asking(focus: Focus, effects: Vec<FocusOut>) -> Outcome {
        Outcome {
            tail: effects,
            ..Outcome::of(focus)
        }
    }

    /// The same, raising `windows` as well.
    pub(super) fn raising(self, windows: Vec<WindowId>) -> Outcome {
        Outcome {
            raise: windows,
            ..self
        }
    }

    /// The same, routing a press.
    pub(super) fn routing(self, route: PressRoute) -> Outcome {
        Outcome {
            route: Some(route),
            ..self
        }
    }

    /// The final focus and every effect, the move's own derived from `before`.
    pub(super) fn finish(self, before: &Focus, view: &FocusView) -> (Focus, Vec<FocusOut>) {
        let after = &self.focus;
        let raise = (!self.raise.is_empty()).then(|| FocusOut::Raise(self.raise.clone()));
        let route = self.route.map(FocusOut::Route);
        let keyboard = (before.key != after.key).then_some(FocusOut::Keyboard(after.key));
        let emphasis = emphasis_changes(before, after, view);
        let emphasis = (!emphasis.is_empty()).then_some(FocusOut::Emphasis(emphasis));
        let active =
            (before.active != after.active).then(|| FocusOut::ActiveApp(after.active.clone()));
        let out = [raise, route, keyboard, emphasis, active]
            .into_iter()
            .flatten()
            .chain(self.tail)
            .collect();
        (self.focus, out)
    }
}

/// How a window looks under `focus`: key, main, or inactive (A2).
fn emphasis_of(focus: &Focus, window: WindowId) -> Emphasis {
    if focus.key == Some(window) {
        Emphasis::Key
    } else if focus.main == Some(window) {
        Emphasis::Main
    } else {
        Emphasis::Inactive
    }
}

/// The windows still in the view whose emphasis differs between the two focuses, by id.
fn emphasis_changes(before: &Focus, after: &Focus, view: &FocusView) -> Vec<(WindowId, Emphasis)> {
    [before.key, before.main, after.key, after.main]
        .into_iter()
        .flatten()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter(|id| view.window(*id).is_some())
        .filter(|id| emphasis_of(before, *id) != emphasis_of(after, *id))
        .map(|id| (id, emphasis_of(after, id)))
        .collect()
}
