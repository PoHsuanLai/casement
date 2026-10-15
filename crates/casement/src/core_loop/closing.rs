//! The frame of a window that has closed. A closing window leaves the model at once, so nothing
//! says how its frame looked; the loop keeps what it needs (the style, the title, who was
//! active) for every framed window while it lives, and draws it from that once the window is
//! gone but motion still has it fading out. `casement-render` draws the frame
//! (`closing_elements`); this decides which windows those are.

use super::frames::activity_of;
use crate::state::State;
use casement_core::WindowId;
use casement_core::window::FrameStyle;
use casement_motion::OutputFrame;
use casement_render::ClosingFrame;
use ds_core::vocab::Activity;
use std::collections::BTreeMap;

/// What is kept of a framed window.
#[derive(Debug, Clone)]
pub(crate) struct Kept {
    /// The frame's look and metrics.
    pub(crate) style: FrameStyle,
    /// The window's title.
    pub(crate) title: String,
    /// Whether it was the window in use.
    pub(crate) activity: Activity,
}

/// Keeps the frame of every window in `styles` (the framed windows of the model) as it is now.
pub(super) fn remember(state: &mut State, styles: &BTreeMap<WindowId, FrameStyle>) {
    let model = &state.core.model;
    for (id, style) in styles {
        if let Some(window) = model.windows().get(id) {
            let kept = Kept {
                style: *style,
                title: window.title.0.clone(),
                activity: activity_of(window),
            };
            state.render.closing.insert(*id, kept);
        }
    }
}

/// Forgets the frames of windows that are neither in the model nor in the scene any more.
pub(super) fn forget(state: &mut State, in_scene: &[WindowId]) {
    let model = &state.core.model;
    state
        .render
        .closing
        .retain(|id, _| model.windows().contains_key(id) || in_scene.contains(id));
}

/// Whether a closing window's frame is drawn while it leaves.
#[derive(Debug, Clone, Copy)]
enum ClosingFrames {
    /// Drawn, fading and shrinking with the window.
    #[allow(dead_code)]
    On,
    /// Not drawn: the window and its frame go at once.
    Off,
}

/// Off until the window's last image is kept with it: the client's surfaces leave the surface book
/// on unmap, so today the frame would fade round an empty body, which reads worse than the window
/// going at once (FINDINGS wave-2026-10-15/flight2).
const CLOSING_FRAMES: ClosingFrames = ClosingFrames::Off;

/// The frames to draw on `frame`: the windows motion still places that the model no longer has.
pub(super) fn on(state: &State, frame: &OutputFrame) -> BTreeMap<WindowId, ClosingFrame> {
    if matches!(CLOSING_FRAMES, ClosingFrames::Off) {
        return BTreeMap::new();
    }
    let placed: Vec<WindowId> = frame.windows.iter().map(|placed| placed.window).collect();
    let model = &state.core.model;
    leaving(&state.render.closing, &placed, |id| {
        model.windows().contains_key(id)
    })
    .map(|(id, kept)| (*id, ClosingFrame::new(kept.style)))
    .collect()
}

/// The entries of `kept` that are in `placed` and no longer `alive`.
fn leaving<'a, T>(
    kept: &'a BTreeMap<WindowId, T>,
    placed: &'a [WindowId],
    alive: impl Fn(&WindowId) -> bool + 'a,
) -> impl Iterator<Item = (&'a WindowId, &'a T)> {
    kept.iter()
        .filter(move |(id, _)| placed.contains(id) && !alive(id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_window_that_is_placed_and_gone_from_the_model_is_leaving() {
        let kept: BTreeMap<WindowId, &str> = [
            (WindowId(1), "alive and placed"),
            (WindowId(2), "closed and placed"),
            (WindowId(3), "closed and finished"),
        ]
        .into_iter()
        .collect();
        let placed = [WindowId(1), WindowId(2), WindowId(9)];
        let alive = |id: &WindowId| *id == WindowId(1);
        let got: Vec<_> = leaving(&kept, &placed, alive).map(|(id, _)| *id).collect();
        assert_eq!(got, vec![WindowId(2)]);
    }

    #[test]
    fn nothing_is_leaving_when_nothing_is_placed() {
        let kept: BTreeMap<WindowId, u8> = [(WindowId(2), 0)].into_iter().collect();
        assert_eq!(leaving(&kept, &[], |_| false).count(), 0);
    }
}
