//! Arming timers on the event loop: a real calloop `Timer` on the wall clock (the virtual
//! clock's book holds timers otherwise: `ClockSource::arm`). calloop's `Timer` reads the wall
//! clock, which is why it lives here (row B8).

use super::source::WallClock;
use crate::core_loop;
use crate::state::State;
use casement_core::time::{Due, Millihertz, VblankClock};
use casement_core::{Event, OutputKey};
use smithay::reexports::calloop::LoopHandle;
use smithay::reexports::calloop::timer::{TimeoutAction, Timer};

/// Arms `due` (`Effect::ArmTimer`) on the wall clock: when it fires, `Event::TimerFired(due.key)`
/// is fed to the core loop, unless the timer was cancelled or armed again for another instant
/// by then.
pub fn arm(handle: &LoopHandle<'static, State>, wall: &WallClock, due: Due) {
    wall.arm(&due);
    let timer = Timer::from_deadline(wall.instant_of(due.at));
    let clock = wall.clone();
    let inserted = handle.insert_source(timer, move |_, (), state: &mut State| {
        if clock.take_if_armed(&due.key, due.at) {
            core_loop::feed(state, vec![Event::TimerFired(due.key.clone())]);
        }
        TimeoutAction::Drop
    });
    if let Err(error) = inserted {
        tracing::warn!(%error, "a timer could not be armed");
    }
}

/// Starts `output`'s vblanks at `refresh` on the wall clock (a headless run with `--clock wall`):
/// a timer that calls `core_loop::frame` at each vblank and stops once the output is gone or
/// another generation of its vblanks has started.
pub fn run_vblanks(
    handle: &LoopHandle<'static, State>,
    wall: &WallClock,
    output: OutputKey,
    refresh: Millihertz,
) {
    let generation = wall.start_vblanks(&output);
    let vblanks = VblankClock::new(refresh, wall.now());
    let first = vblanks.next_after(wall.now()).0;
    let clock = wall.clone();
    let timer = Timer::from_deadline(wall.instant_of(first));
    let inserted = handle.insert_source(timer, move |_, (), state: &mut State| {
        if !clock.vblanks_run(&output, generation) {
            return TimeoutAction::Drop;
        }
        core_loop::frame(state, &output);
        let (next, _) = vblanks.next_after(clock.now());
        TimeoutAction::ToInstant(clock.instant_of(next))
    });
    if let Err(error) = inserted {
        tracing::warn!(%error, "an output's vblanks could not start");
    }
}
