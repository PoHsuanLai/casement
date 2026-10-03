//! Time as data: the session's monotonic instant ([`Mono`]), refresh rates, vblank clocks, the
//! timer book and the virtual clock that steps them in order.
//!
//! Nothing here reads a clock. The composition root's `casement/src/time/` is the only reader
//! (boundary row B8) and wraps [`VirtualClock`] in a calloop event source for headless runs
//! (21-FREEZE-AMENDMENTS A1).

mod millihertz;
mod mono;
mod timers;
mod vblank_clock;
mod virtual_clock;

pub use millihertz::Millihertz;
pub use mono::Mono;
pub use timers::{Due, IdleNotifierId, TimerBook, TimerKey, TrustedKey};
pub use vblank_clock::{VblankClock, VblankSeq};
pub use virtual_clock::{Arrival, Stop, VirtualClock};

#[cfg(test)]
mod stop_order_tests;
#[cfg(test)]
mod tests;
