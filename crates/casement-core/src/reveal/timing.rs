//! The numbers an edge reveal runs on, as data.

use super::{Reduced, RevealEdge};
use std::time::Duration;

/// How long the pointer rests at the top edge before the bar comes (hover intent, design/27
/// section 4.5).
pub const BAR_DWELL: Duration = Duration::from_millis(450);

/// How long the bar takes to slide: `--t-move` of the timing table (`DurationToken::Move`).
pub const BAR_SLIDE: Duration = Duration::from_millis(200);

/// How long the pointer rests at the bottom edge before the dock comes: `dock.autohide_delay_ms`
/// at its settled default (design/10 section 10.3.11).
pub const DOCK_DWELL: Duration = Duration::from_millis(200);

/// How long the dock takes to slide: `dock.autohide_slide_ms` at its settled default.
pub const DOCK_SLIDE: Duration = Duration::from_millis(500);

/// How long the cross-fade takes under Reduced motion: `--t-quick`, which every moving duration
/// becomes (`DurationToken::Move` at the Reduced level).
pub const REDUCED_FADE: Duration = Duration::from_millis(120);

/// How the surface comes and goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Look {
    /// It slides in from its edge.
    Slide,
    /// It fades in place (Reduced motion: no state is carried by movement alone).
    CrossFade,
}

/// One edge's dwell and travel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RevealTiming {
    /// How long the pointer must rest in the strip. Intent, not motion: Reduced does not
    /// shorten it.
    pub dwell: Duration,
    /// How long it takes to come or to go.
    pub travel: Duration,
    /// Whether it slides or fades.
    pub look: Look,
}

impl RevealTiming {
    /// The timing of `edge` at the motion level `reduced` says.
    ///
    /// ```
    /// use casement_core::reveal::{BAR_DWELL, Look, Reduced, RevealEdge, RevealTiming};
    ///
    /// let timing = RevealTiming::of(RevealEdge::Top, Reduced::No);
    /// assert_eq!((timing.dwell, timing.look), (BAR_DWELL, Look::Slide));
    /// ```
    pub fn of(edge: RevealEdge, reduced: Reduced) -> RevealTiming {
        let (dwell, slide) = match edge {
            RevealEdge::Top => (BAR_DWELL, BAR_SLIDE),
            RevealEdge::Bottom => (DOCK_DWELL, DOCK_SLIDE),
        };
        match reduced {
            Reduced::No => RevealTiming {
                dwell,
                travel: slide,
                look: Look::Slide,
            },
            Reduced::Yes => RevealTiming {
                dwell,
                travel: REDUCED_FADE,
                look: Look::CrossFade,
            },
        }
    }
}
