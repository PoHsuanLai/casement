//! The watchdog's ping: a calloop timer on the loop that sends `Notice::Watchdog`, so the
//! service manager hears nothing when the loop stops turning (which is what it is for). It lives
//! here because calloop's `Timer` reads the wall clock (row B8).

use crate::core_loop::tty::display_health;
use crate::notify::Notice;
use crate::state::State;
use casement_backend::tty::Health;
use smithay::reexports::calloop::LoopHandle;
use smithay::reexports::calloop::timer::{TimeoutAction, Timer};
use std::time::Duration;

/// Pings every `every` for as long as the loop runs and the display is showing frames: a screen
/// that has shown nothing for long on every output (a lost page flip, a GPU that reset) is a
/// dead session however well the loop turns, so no ping is sent and the service manager
/// restarts it. A ping that cannot be sent is logged.
pub fn ping_every(handle: &LoopHandle<'static, State>, every: Duration) {
    let inserted = handle.insert_source(
        Timer::from_duration(every),
        move |_, (), state: &mut State| {
            match display_health(state) {
                Health::Alive => {
                    if let Err(error) = state.seams.notify.send(&Notice::Watchdog) {
                        tracing::warn!(%error, "the watchdog ping could not be sent");
                    }
                }
                Health::Frozen => {
                    tracing::error!(
                        "no screen has shown a frame for a long while; the watchdog is not fed"
                    );
                }
                Health::SeatLost => {
                    tracing::error!("the seat is gone; the watchdog is not fed");
                }
            }
            TimeoutAction::ToDuration(every)
        },
    );
    if let Err(error) = inserted {
        tracing::warn!(%error, "the watchdog timer could not be armed");
    }
}
