//! The watchdog's ping: a calloop timer on the loop that sends `Notice::Watchdog`, so the
//! service manager hears nothing when the loop stops turning (which is what it is for). It lives
//! here because calloop's `Timer` reads the wall clock (row B8).

use crate::notify::Notice;
use crate::state::State;
use smithay::reexports::calloop::LoopHandle;
use smithay::reexports::calloop::timer::{TimeoutAction, Timer};
use std::time::Duration;

/// Pings every `every` for as long as the loop runs. A ping that cannot be sent is logged.
pub fn ping_every(handle: &LoopHandle<'static, State>, every: Duration) {
    let inserted = handle.insert_source(
        Timer::from_duration(every),
        move |_, (), state: &mut State| {
            if let Err(error) = state.seams.notify.send(&Notice::Watchdog) {
                tracing::warn!(%error, "the watchdog ping could not be sent");
            }
            TimeoutAction::ToDuration(every)
        },
    );
    if let Err(error) = inserted {
        tracing::warn!(%error, "the watchdog timer could not be armed");
    }
}
