//! The corners and the shadow casement gives a window that draws its own frame (a quire app, a
//! GTK headerbar window, an Electron one). A window casement frames itself has both from its
//! frame; this is for the rest.

use super::hints::FrameChoice;

/// Whether a window's buffer reaches past its xdg window geometry: a toolkit that draws its own
/// shadow (GTK) puts it in the margin outside the geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Margin {
    /// The geometry is the whole buffer.
    None,
    /// The buffer is larger than the geometry: the client has drawn something round the window.
    Drawn,
}

/// Who casts the shadow of a window casement rounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cast {
    /// Casement, with the shadow its framed windows have.
    Casement,
    /// The client, in its own buffer.
    Client,
}

/// How casement shapes a window that draws its own frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Round {
    /// The corner radius, in logical pixels, at the window geometry's corners.
    pub radius: u32,
    /// Who casts the shadow.
    pub shadow: Cast,
}

/// What casement does for a client-framed window, from what the client said (`choice`), whether
/// its buffer holds a margin (`margin`) and the desktop's window radius (`desktop`):
///
/// - `set_frame(compositor_shadow, r)` asks for both: corners of `r`, casement's shadow.
/// - `set_frame(client, _)` draws its own corners and shadow: casement does nothing.
/// - A client that said nothing gets the desktop's radius, because it will not round itself. The
///   shadow depends on the margin: a toolkit that drew a margin has drawn its own shadow in it
///   (two would look heavy), so casement casts none; a window whose geometry is its whole buffer
///   has no shadow of its own, so casement casts one.
///
/// ```
/// use casement_core::window::{Cast, FrameChoice, Margin, Round, round_of};
///
/// let silent = |margin| round_of(FrameChoice::Silent, margin, 10);
/// assert_eq!(silent(Margin::None), Some(Round { radius: 10, shadow: Cast::Casement }));
/// assert_eq!(silent(Margin::Drawn), Some(Round { radius: 10, shadow: Cast::Client }));
/// assert_eq!(round_of(FrameChoice::Own, Margin::None, 10), None);
/// ```
pub fn round_of(choice: FrameChoice, margin: Margin, desktop: u32) -> Option<Round> {
    match (choice, margin) {
        (FrameChoice::Own, _) => None,
        (FrameChoice::Shadowed { radius }, _) => Some(Round {
            radius,
            shadow: Cast::Casement,
        }),
        (FrameChoice::Silent, Margin::None) => Some(Round {
            radius: desktop,
            shadow: Cast::Casement,
        }),
        (FrameChoice::Silent, Margin::Drawn) => Some(Round {
            radius: desktop,
            shadow: Cast::Client,
        }),
    }
}
