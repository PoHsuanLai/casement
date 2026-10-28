//! Edge reveal: in full screen the bar (top edge) and the dock (bottom edge) come over the
//! full-screen window when the pointer rests at their edge, and go away when it leaves them.
//!
//! One pure machine per output and edge ([`Reveal`], kept in a [`RevealBook`]). The edge feeds
//! it the pointer's place, whether a pop-up of the surface is open, whether the output shows a
//! full-screen Space, and the timer it asked for; it answers with the timer to arm or cancel and
//! the `reveal` note to tell the shell. What is drawn follows from the state and the clock:
//! [`Reveal::stacking`] says whether the surface is above the full-screen window,
//! [`Reveal::visible`] how far in it is, and [`slid`] where that puts it. Nothing here reads a
//! clock, and nothing here touches the full-screen window: the bar and the dock overlay it and
//! take no room from it.
//!
//! The numbers are data ([`RevealTiming`]): the bar waits 450 ms (design/27 section 4.5) and
//! slides for the move duration of the timing table; the dock keeps its own auto-hide numbers
//! (design/10 section 10.3.11); under Reduced motion both cross-fade for the quick duration.

mod book;
mod edge;
mod machine;
mod slide;
mod timing;

pub use book::RevealBook;
pub use casement_protocols::vocab::FullScreenBar;
pub use edge::{Place, RevealEdge, STRIP};
pub use machine::{Reveal, RevealIn, RevealOut};
pub use slide::{Slid, slid};
pub use timing::{BAR_DWELL, BAR_SLIDE, DOCK_DWELL, DOCK_SLIDE, Look, REDUCED_FADE, RevealTiming};

#[cfg(test)]
mod machine_tests;
#[cfg(test)]
mod slide_tests;
#[cfg(test)]
mod timing_tests;
