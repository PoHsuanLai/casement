//! The only clock readers in casement (row B8): `Instant::now`, smithay's `Clock::now` and
//! calloop's `Timer` appear nowhere else. The model takes `Mono` as an argument; this module
//! decides what `Mono` is.
//!
//! The virtual clock (tests, `--clock stepped`) is casement-core's `VirtualClock` shared with
//! a calloop event source ([`VirtualTimerSource`], 21-FREEZE-AMENDMENTS A1, SP3): calloop's own
//! timers read the wall clock and cannot be virtualised.

mod calloop;
mod local_minute;
mod monotonic;
mod source;
mod virtual_clock;
mod wall;
mod watchdog;

pub use calloop::{after, arm};
pub use local_minute::{minute_of, tick_every_minute, until_next_minute};
pub use monotonic::monotonic_ns;
pub use source::ClockSource;
pub use virtual_clock::{SharedClock, VirtualTimerSource};
pub use wall::WallClock;
pub use watchdog::ping_every;
