//! Page-flip pacing of a real scanout surface (pure): one flip in flight at a time, and the
//! vblank each queued frame is for. The GLES surface (`surface_real.rs`) and the pixman surface
//! (`soft_real.rs`) count the same way, so a frame reports the same presentation on either.
//!
//! The count is the surface's own, not the kernel's sequence number: a missed vblank is not
//! measured on real hardware (FINDINGS "p1-t").

use super::surface::Presented;
use casement_core::time::{Mono, VblankSeq};

/// Whether a frame may be queued now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Turn {
    /// No flip is in flight: a frame queued now is for this vblank.
    Free(VblankSeq),
    /// A flip is in flight; try at the next vblank.
    Busy,
}

/// One surface's flip count.
///
/// ```
/// use casement_backend::tty::{Pace, Turn};
/// use casement_core::time::{Mono, VblankSeq};
/// use std::time::Duration;
///
/// let mut pace = Pace::new();
/// assert_eq!(pace.turn(), Turn::Free(VblankSeq(0)));
/// pace.queued();
/// assert_eq!(pace.turn(), Turn::Busy);
/// let shown = pace.landed(Mono(Duration::ZERO)).expect("a flip was in flight");
/// assert_eq!(shown.seq, VblankSeq(0));
/// assert_eq!(pace.turn(), Turn::Free(VblankSeq(1)));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pace {
    /// The vblank the next queued frame is for.
    next: VblankSeq,
    /// The vblank of the frame whose flip is in flight.
    pending: Option<VblankSeq>,
}

impl Default for Pace {
    fn default() -> Pace {
        Pace::new()
    }
}

impl Pace {
    /// A count with nothing queued, starting at vblank 0.
    pub fn new() -> Pace {
        Pace {
            next: VblankSeq(0),
            pending: None,
        }
    }

    /// Whether a frame may be queued, and for which vblank.
    pub fn turn(&self) -> Turn {
        match self.pending {
            Some(_) => Turn::Busy,
            None => Turn::Free(self.next),
        }
    }

    /// The vblank of the frame in flight, if one is.
    pub fn pending(&self) -> Option<VblankSeq> {
        self.pending
    }

    /// A frame was queued for the vblank [`turn`](Self::turn) named.
    pub fn queued(&mut self) {
        self.pending = Some(self.next);
    }

    /// The flip in flight finished at `now`: the frame that reached the glass, and the count
    /// moves on to the next vblank. None when no flip was in flight.
    pub fn landed(&mut self, now: Mono) -> Option<Presented> {
        let seq = self.pending.take()?;
        self.next = VblankSeq(seq.0.saturating_add(1));
        Some(Presented::new(seq, now))
    }

    /// The flip in flight is lost (the output went dark, the seat came back): the next frame is
    /// for the vblank after the lost one's, and nothing is pending.
    pub fn lost(&mut self) {
        self.pending = None;
    }
}
