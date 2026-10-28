//! The last image of a window that is about to close, so a closing window fades and shrinks as one
//! (its body with its frame). A client's surfaces leave the surface book the moment it unmaps, so
//! the loop draws the window alone, through the same offscreen pass window capture uses, just
//! before: when a commit takes the buffer away (a pre-commit hook) or when the toplevel is
//! destroyed with its buffer still attached. The pixels live in [`Bodies`] until the window has
//! left the motion scene; the number and the weight of what is kept are capped.

use super::closing::Kept;
use crate::handlers::capture_source::{body_copy, content_and_scale, window_rgba, window_texture};
use crate::state::State;
use casement_backend::tty::BodyCopy;
use casement_core::WindowId;
use casement_core::geometry::LogicalSize;
use casement_render::ClosingBody;
use ds_core::geometry::scale::Scale;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::wayland::compositor::{BufferAssignment, SurfaceAttributes, with_states};
use std::collections::VecDeque;

/// Most images kept at once (windows closing together, e.g. an app quitting).
const MAX_KEPT: usize = 6;

/// Most bytes kept at once: a 4K window is about 33 MB.
const MAX_BYTES: usize = 192 * 1024 * 1024;

/// What decides whether a window's last image is worth taking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BodyChoice {
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
    pub(super) framed: bool,
    /// Its content size.
    pub(super) size: LogicalSize,
    /// Its output's scale in 120ths (`Scale::numerator`: 120 is 1x).
    pub(super) units: u32,
    /// Whether an image of it is kept already.
    pub(super) imaged: bool,
}

/// Whether to take the last image of the window `facts` describe. Motion's departure fades every
/// framed window, whatever the motion level (reduced motion only drops the shrink), so every
/// framed window animates.
pub(super) fn verdict(facts: Facts) -> BodyChoice {
    let pixels = |side: u32| u64::from(side) * u64::from(facts.units) / 120;
    let bytes = pixels(facts.size.w)
        .saturating_mul(pixels(facts.size.h))
        .saturating_mul(4);
    match (facts.framed, facts.imaged) {
        (false, _) => BodyChoice::Unframed,
        (_, true) => BodyChoice::Held,
        _ if facts.size.w == 0 || facts.size.h == 0 => BodyChoice::Empty,
        _ if bytes > u64::try_from(MAX_BYTES).unwrap_or(u64::MAX) => BodyChoice::TooBig,
        _ => BodyChoice::Take,
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
    fn has_image(&self, window: &WindowId) -> bool {
        self.get(window).is_some()
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

    /// Frees the images of the windows `keep` refuses (the ones whose animation is over); a GPU
    /// texture goes with its body.
    pub(super) fn retain(&mut self, keep: impl Fn(&WindowId) -> bool) {
        self.kept.retain(|(id, _)| keep(id));
    }

    /// Frees the images that are not for a window gone from the model that motion still fades:
    /// one whose window is back (a client that hid and re-showed it), or whose fade is over.
    pub(super) fn release_finished(
        &mut self,
        in_model: impl Fn(&WindowId) -> bool,
        in_scene: impl Fn(&WindowId) -> bool,
    ) {
        self.retain(|id| !in_model(id) && in_scene(id));
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
        framed: style.is_some(),
        size,
        units: scale.numerator(),
        imaged: state.render.bodies.has_image(&window),
    };
    let (BodyChoice::Take, Some(style)) = (verdict(facts), style) else {
        return;
    };
    let id = state.render.bodies.next_id();
    let Some(body) = make_body(state, window, id, (style.look.radius, scale)) else {
        return;
    };
    state.render.bodies.admit(window, body);
}

/// The last image of `window` as a body: copied into a texture on the display GPU where the seat
/// allows ([`body_copy`]), no read-back; read back through the offscreen pass otherwise, and when
/// the GPU copy fails.
fn make_body(
    state: &mut State,
    window: WindowId,
    id: u64,
    (radius, scale): (u32, Scale),
) -> Option<ClosingBody> {
    let on_gpu = match body_copy(state) {
        BodyCopy::Gpu => window_texture(state, window, radius)
            .ok()
            .map(|(texture, size)| ClosingBody::from_texture(id, size, texture)),
        _ => None,
    };
    on_gpu.or_else(|| {
        let frame = window_rgba(state, window).ok()?;
        ClosingBody::from_rgba(id, frame.size, &frame.bytes, (radius, scale))
    })
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

    fn facts(framed: bool, (w, h): (u32, u32), imaged: bool) -> Facts {
        Facts {
            framed,
            size: LogicalSize { w, h },
            units: 120,
            imaged,
        }
    }

    fn body(id: u64, side: u32) -> ClosingBody {
        let size = PhysicalSize { w: side, h: side };
        let bytes = vec![0; (side * side * 4) as usize];
        ClosingBody::from_rgba(id, size, &bytes, (0, Scale(120))).expect("a body")
    }

    #[test]
    fn only_a_framed_sized_window_without_an_image_is_taken() {
        let (yes, no, absent, present) = (true, false, false, true);
        // name, facts, verdict
        let table = [
            ("framed", facts(yes, (800, 600), absent), BodyChoice::Take),
            (
                "no frame",
                facts(no, (800, 600), absent),
                BodyChoice::Unframed,
            ),
            ("already", facts(yes, (800, 600), present), BodyChoice::Held),
            ("no width", facts(yes, (0, 600), absent), BodyChoice::Empty),
            ("no height", facts(yes, (800, 0), absent), BodyChoice::Empty),
            (
                "too big",
                facts(yes, (9_000, 9_000), absent),
                BodyChoice::TooBig,
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
            ..facts(true, (5_000, 4_000), false)
        };
        assert_eq!(verdict(at(120)), BodyChoice::Take);
        assert_eq!(verdict(at(240)), BodyChoice::TooBig);
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
    fn an_image_is_freed_when_its_fade_is_over_or_its_window_is_back() {
        let mut bodies = Bodies::default();
        for n in 1..=3 {
            bodies.admit(WindowId(n), body(n, 2));
        }
        // 1 is fading, 2 is back in the model, 3 has left the scene (its fade is over).
        bodies.release_finished(|id| *id == WindowId(2), |id| *id != WindowId(3));
        assert_eq!(bodies.len(), 1);
        assert!(bodies.get(&WindowId(1)).is_some());
    }
}
