//! Mission Control under the fingers: while a vertical swipe is down, the windows are laid out
//! at the swipe's own progress (`Model::overview_progress`) instead of flying the timed 350 ms
//! flight. Each step snaps motion to the spread mixed to that progress, so the book of motion
//! holds the poses the fingers chose; the lift then retargets from there (a commit flies on to
//! the slots, a snap-back returns), which is what continuing from the progress means. Keyboard
//! and hot-corner opens never come here and keep the timed flight.

use super::frames;
use crate::state::State;
use casement_core::Cause;
use casement_core::PageMilli;
use casement_core::overview::Overview;
use casement_layout::{Arrangement, SceneLayout, spec_of_framed};
use casement_motion::{MotionBook, retarget};
use ds_core::vocab::Fraction;

/// A full opening, in thousandths.
const OPEN: i32 = 1000;

/// `progress` as the layout's openness, held to `0..=1000`.
pub(super) fn openness(progress: PageMilli) -> Fraction {
    Fraction(u16::try_from(progress.0.clamp(0, OPEN)).unwrap_or(0))
}

/// The windows follow the fingers: when a vertical swipe is down, motion is snapped to the
/// spread at its progress. Nothing otherwise.
pub(super) fn follow(state: &mut State) {
    let Some(progress) = state.core.model.overview_progress() else {
        return;
    };
    let now = state.clock.now();
    let spread = spread_of(state);
    let followed = spread.spread_at(openness(progress));
    let motion = std::mem::replace(&mut state.core.motion, MotionBook::empty());
    state.core.motion = retarget(
        motion,
        &state.core.arrangement,
        &followed,
        &Cause::Configure,
        Default::default(),
        now,
    );
}

/// The arrangement with the shown Spaces' windows in their slots: the current one while
/// Mission Control is up, else the one it would have.
fn spread_of(state: &State) -> Arrangement {
    match state.core.model.overview() {
        Overview::Entering | Overview::Open => state.core.arrangement.clone(),
        Overview::Closed | Overview::Leaving => {
            let spec =
                spec_of_framed(&state.core.model, &frames::reserves(state)).in_mission_control();
            state.core.layout.arrange(&spec, &state.core.knobs)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_is_held_to_the_range() {
        let cases = [(-50, 0), (0, 0), (500, 500), (1000, 1000), (1300, 1000)];
        for (given, want) in cases {
            assert_eq!(openness(PageMilli(given)), Fraction(want), "{given}");
        }
    }
}
