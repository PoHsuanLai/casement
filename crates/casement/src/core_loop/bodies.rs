//! The last image of a window that is about to close, so a closing window fades and shrinks as one
//! (its body with its frame). A client's surfaces leave the surface book the moment it unmaps, so
//! the loop draws the window alone, through the same offscreen pass window capture uses, just
//! before: when a commit takes the buffer away (a pre-commit hook) or when the toplevel is
//! destroyed with its buffer still attached. The pixels live in [`Bodies`] until the window has
//! left the motion scene; the number and the weight of what is kept are capped.

use super::closing::Kept;
use crate::handlers::capture_source::{content_and_scale, window_rgba};
use crate::state::State;
use casement_core::WindowId;
use casement_core::geometry::LogicalSize;
use casement_render::ClosingBody;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::wayland::compositor::{BufferAssignment, SurfaceAttributes, with_states};
use std::collections::VecDeque;

/// Most images kept at once (windows closing together, e.g. an app quitting).
const MAX_KEPT: usize = 6;

/// Most bytes kept at once: a 4K window is about 33 MB.
const MAX_BYTES: usize = 192 * 1024 * 1024;

/// What decides whether a window's last image is worth taking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Verdict {
    /// Take it.
    Take,
    /// The window has no frame of ours: nothing is drawn for it as it closes.
    Unframed,
    /// The window has no size to draw.
    Empty,
    /// The image alone would not fit the cap.
    TooBig,
    /// One is kept already.
    Held,
}

/// What is known of a window that may close.
#[derive(Debug, Clone, Copy)]
pub(super) struct Facts {
    /// Whether the loop kept a frame for it (only framed windows get a closing animation).
    pub(super) frame: FrameKept,
    /// Its content size.
    pub(super) size: LogicalSize,
    /// Its output's scale in 120ths (`Scale::numerator`: 120 is 1x).
    pub(super) units: u32,
    /// Whether an image of it is kept already.
    pub(super) image: ImageKept,
}

/// Whether the loop kept a frame for a window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FrameKept {
    /// It did.
    Yes,
    /// It did not.
    No,
}

/// Whether an image is kept for a window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ImageKept {
    /// It is.
    Yes,
    /// It is not.
    No,
}

/// Whether to take the last image of the window `facts` describe. Motion's departure fades every
/// framed window, whatever the motion level (reduced motion only drops the shrink), so every
/// framed window animates.
pub(super) fn verdict(facts: Facts) -> Verdict {
    let pixels = |side: u32| u64::from(side) * u64::from(facts.units) / 120;
    let bytes = pixels(facts.size.w)
        .saturating_mul(pixels(facts.size.h))
        .saturating_mul(4);
    match (facts.frame, facts.image) {
        (FrameKept::No, _) => Verdict::Unframed,
        (_, ImageKept::Yes) => Verdict::Held,
        _ if facts.size.w == 0 || facts.size.h == 0 => Verdict::Empty,
        _ if bytes > u64::try_from(MAX_BYTES).unwrap_or(u64::MAX) => Verdict::TooBig,
        _ => Verdict::Take,
    }
}

/// The images kept for windows that are closing, oldest first.
#[derive(Debug, Default)]
pub(crate) struct Bodies {
    kept: VecDeque<(WindowId, ClosingBody)>,
    made: u64,
}

impl Bodies {
    /// The image kept for `window`.
    pub(crate) fn get(&self, window: &WindowId) -> Option<&ClosingBody> {
        self.kept
            .iter()
            .find(|(id, _)| id == window)
            .map(|(_, body)| body)
    }

    /// Whether an image is kept for `window`.
    fn image_of(&self, window: &WindowId) -> ImageKept {
        match self.get(window) {
            Some(_) => ImageKept::Yes,
            None => ImageKept::No,
        }
    }

    /// The id the next image gets.
    fn next_id(&mut self) -> u64 {
        self.made = self.made.wrapping_add(1);
        self.made
    }

    /// Keeps `body` for `window`, dropping the oldest images while there are more than
    /// [`MAX_KEPT`] or more than [`MAX_BYTES`]; the windows dropped.
    fn admit(&mut self, window: WindowId, body: ClosingBody) -> Vec<WindowId> {
        self.kept.retain(|(id, _)| *id != window);
        self.kept.push_back((window, body));
        let mut dropped = Vec::new();
        while self.kept.len() > MAX_KEPT || self.weight() > MAX_BYTES {
            match self.kept.pop_front() {
                Some((id, _)) if self.kept.is_empty() => {
                    // The one just admitted does not fit alone: nothing is kept.
                    dropped.push(id);
                    break;
                }
                Some((id, _)) => dropped.push(id),
                None => break,
            }
        }
        dropped
    }

    /// What the images weigh together.
    fn weight(&self) -> usize {
        self.kept.iter().map(|(_, body)| body.bytes()).sum()
    }

    /// Frees the images of the windows `keep` refuses (the ones whose animation is over).
    pub(super) fn retain(&mut self, keep: impl Fn(&WindowId) -> bool) {
        self.kept.retain(|(id, _)| keep(id));
    }

    /// How many images are kept.
    #[cfg(test)]
    fn len(&self) -> usize {
        self.kept.len()
    }
}

/// Draws `window` alone and keeps its image, when [`verdict`] says to. A window the backend cannot
/// draw offscreen (a winit window) keeps nothing, and its frame goes at once as before.
pub(super) fn take(state: &mut State, window: WindowId) {
    let style = state
        .render
        .closing
        .get(&window)
        .map(|Kept { style, .. }| *style);
    let Some((size, scale)) = content_and_scale(state, window) else {
        return;
    };
    let facts = Facts {
        frame: style.map_or(FrameKept::No, |_| FrameKept::Yes),
        size,
        units: scale.numerator(),
        image: state.render.bodies.image_of(&window),
    };
    let (Verdict::Take, Some(style)) = (verdict(facts), style) else {
        return;
    };
    let Ok(frame) = window_rgba(state, window) else {
        return;
    };
    let id = state.render.bodies.next_id();
    let Some(body) =
        ClosingBody::from_rgba(id, frame.size, &frame.bytes, (style.look.radius, scale))
    else {
        return;
    };
    state.render.bodies.admit(window, body);
}

/// A commit is about to take the buffer off `surface`: if it is a mapped toplevel's root, its
/// image is taken now, while the old buffer is still the window's.
pub(crate) fn before_commit(state: &mut State, surface: &WlSurface) {
    let removes = with_states(surface, |states| {
        matches!(
            states
                .cached_state
                .get::<SurfaceAttributes>()
                .pending()
                .buffer,
            Some(BufferAssignment::Removed)
        )
    });
    if !removes {
        return;
    }
    let Some(window) = state.wl.xdg_shell.window_of_surface(surface) else {
        return;
    };
    // Only a window the book draws is mapped; an unmapped one has nothing to keep.
    if state.render.textures.window(&window).is_some() {
        take(state, window);
    }
}

/// A toplevel is being destroyed, its buffer perhaps still attached: its image is taken now.
pub(crate) fn before_destroy(state: &mut State, window: WindowId) {
    if state.render.textures.window(&window).is_some() {
        take(state, window);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use casement_core::geometry::PhysicalSize;
    use ds_core::geometry::scale::Scale;

    fn facts(frame: FrameKept, (w, h): (u32, u32), image: ImageKept) -> Facts {
        Facts {
            frame,
            size: LogicalSize { w, h },
            units: 120,
            image,
        }
    }

    fn body(id: u64, side: u32) -> ClosingBody {
        let size = PhysicalSize { w: side, h: side };
        let bytes = vec![0; (side * side * 4) as usize];
        ClosingBody::from_rgba(id, size, &bytes, (0, Scale(120))).expect("a body")
    }

    #[test]
    fn only_a_framed_sized_window_without_an_image_is_taken() {
        use FrameKept::{No, Yes};
        use ImageKept::{No as Absent, Yes as Present};
        // name, facts, verdict
        let table = [
            ("framed", facts(Yes, (800, 600), Absent), Verdict::Take),
            ("no frame", facts(No, (800, 600), Absent), Verdict::Unframed),
            ("already", facts(Yes, (800, 600), Present), Verdict::Held),
            ("no width", facts(Yes, (0, 600), Absent), Verdict::Empty),
            ("no height", facts(Yes, (800, 0), Absent), Verdict::Empty),
            (
                "too big",
                facts(Yes, (9_000, 9_000), Absent),
                Verdict::TooBig,
            ),
        ];
        for (name, input, want) in table {
            assert_eq!(verdict(input), want, "{name}");
        }
    }

    #[test]
    fn the_scale_counts_in_the_weight() {
        // 5000 x 4000 logical is 80 MB at 1x and 320 MB at 2x.
        let at = |units| Facts {
            units,
            ..facts(FrameKept::Yes, (5_000, 4_000), ImageKept::No)
        };
        assert_eq!(verdict(at(120)), Verdict::Take);
        assert_eq!(verdict(at(240)), Verdict::TooBig);
    }

    #[test]
    fn the_oldest_images_go_first_past_the_count() {
        let mut bodies = Bodies::default();
        let mut dropped = Vec::new();
        for n in 0..=MAX_KEPT as u64 {
            dropped.extend(bodies.admit(WindowId(n), body(n, 2)));
        }
        assert_eq!(dropped, vec![WindowId(0)]);
        assert_eq!(bodies.len(), MAX_KEPT);
        assert!(bodies.get(&WindowId(0)).is_none());
        assert!(bodies.get(&WindowId(1)).is_some());
    }

    #[test]
    fn a_window_admitted_again_replaces_its_image() {
        let mut bodies = Bodies::default();
        bodies.admit(WindowId(1), body(1, 2));
        let dropped = bodies.admit(WindowId(1), body(2, 2));
        assert!(dropped.is_empty());
        assert_eq!(bodies.len(), 1);
        assert_eq!(bodies.get(&WindowId(1)), Some(&body(2, 2)));
    }

    #[test]
    fn the_images_of_finished_windows_are_freed() {
        let mut bodies = Bodies::default();
        for n in 1..=3 {
            bodies.admit(WindowId(n), body(n, 2));
        }
        bodies.retain(|id| *id == WindowId(2));
        assert_eq!(bodies.len(), 1);
        assert!(bodies.get(&WindowId(2)).is_some());
    }
}
