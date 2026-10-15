//! Hover intent on the green light: the pointer resting on it for about half a second asks for
//! the window menu, as on the Mac. Pure; the edge feeds it the pointer and the clock, arms the
//! timer it asks for, and shows the menu when it says so. The timing is data.

use super::frame::Light;
use super::frame_arm::LightAt;
use crate::time::Mono;
use std::time::Duration;

/// How long the pointer rests on the green light before the menu shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DwellTiming {
    /// The rest.
    pub rest: Duration,
}

impl DwellTiming {
    /// The Mac's: about half a second.
    pub const MAC: DwellTiming = DwellTiming {
        rest: Duration::from_millis(500),
    };
}

impl Default for DwellTiming {
    fn default() -> DwellTiming {
        DwellTiming::MAC
    }
}

/// What the dwell machine is doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum Dwell {
    /// The pointer is not on a green light, or the menu was asked for already and it moved.
    #[default]
    Idle,
    /// The pointer came onto the green light at `since` and has not left it.
    Waiting {
        /// The light.
        on: LightAt,
        /// When the pointer arrived.
        since: Mono,
    },
    /// The menu was asked for; it is not asked for again until the pointer leaves the light.
    Shown(LightAt),
    /// The light was clicked; no menu comes until the pointer leaves it.
    Spent(LightAt),
}

/// What happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DwellInput {
    /// The pointer is at `Mono` over this light, or over no light.
    Over(Option<LightAt>, Mono),
    /// The timer the machine asked for fired.
    Tick(Mono),
    /// A button went down on the light.
    Pressed,
    /// The window or its frame is gone.
    Lost,
}

/// What the edge does about a step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DwellOutcome {
    /// Nothing.
    Nothing,
    /// Arm the dwell timer for this instant (replacing an earlier one).
    Arm(Mono),
    /// Cancel the dwell timer.
    Cancel,
    /// Show the window menu for this window's green light.
    Show(LightAt),
}

impl Dwell {
    /// The state after `input`, and what to do about it. Only the zoom (green) light counts;
    /// a pointer on another light is a pointer on none.
    ///
    /// ```
    /// use casement_core::WindowId;
    /// use casement_core::time::Mono;
    /// use casement_core::window::{Dwell, DwellInput, DwellOutcome, DwellTiming, Light, LightAt};
    /// use std::time::Duration;
    ///
    /// let on = LightAt { window: WindowId(1), light: Light::Zoom };
    /// let t0 = Mono(Duration::from_secs(1));
    /// let (dwell, out) = Dwell::Idle.step(DwellTiming::MAC, DwellInput::Over(Some(on), t0));
    /// let due = t0.after(DwellTiming::MAC.rest);
    /// assert_eq!(out, DwellOutcome::Arm(due));
    /// assert_eq!(dwell.step(DwellTiming::MAC, DwellInput::Tick(due)).1, DwellOutcome::Show(on));
    /// ```
    pub fn step(self, timing: DwellTiming, input: DwellInput) -> (Dwell, DwellOutcome) {
        match (self, input) {
            (_, DwellInput::Lost) => (Dwell::Idle, cancel_if_waiting(self)),
            (Dwell::Waiting { .. }, DwellInput::Pressed) => (Dwell::Idle, DwellOutcome::Cancel),
            (Dwell::Shown(on), DwellInput::Pressed) => (Dwell::Spent(on), DwellOutcome::Nothing),
            (_, DwellInput::Pressed) => (self, DwellOutcome::Nothing),
            (Dwell::Waiting { on, since }, DwellInput::Tick(now))
                if now >= since.after(timing.rest) =>
            {
                (Dwell::Shown(on), DwellOutcome::Show(on))
            }
            (_, DwellInput::Tick(_)) => (self, DwellOutcome::Nothing),
            (_, DwellInput::Over(over, now)) => self.over(timing, green(over), now),
        }
    }

    /// The pointer is over `over` (a green light, or none) at `now`.
    fn over(self, timing: DwellTiming, over: Option<LightAt>, now: Mono) -> (Dwell, DwellOutcome) {
        match (self, over) {
            (Dwell::Waiting { on, .. } | Dwell::Shown(on) | Dwell::Spent(on), Some(here))
                if on == here =>
            {
                (self, DwellOutcome::Nothing)
            }
            (_, Some(here)) => (
                Dwell::Waiting {
                    on: here,
                    since: now,
                },
                DwellOutcome::Arm(now.after(timing.rest)),
            ),
            (_, None) => (Dwell::Idle, cancel_if_waiting(self)),
        }
    }
}

/// The only light with a menu.
fn green(over: Option<LightAt>) -> Option<LightAt> {
    over.filter(|at| at.light == Light::Zoom)
}

/// Cancelling the timer is only worth asking for when one is armed.
fn cancel_if_waiting(state: Dwell) -> DwellOutcome {
    match state {
        Dwell::Waiting { .. } => DwellOutcome::Cancel,
        _ => DwellOutcome::Nothing,
    }
}

#[cfg(test)]
mod tests;
