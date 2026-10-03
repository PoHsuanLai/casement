//! The session clock.

use super::virtual_clock::SharedClock;
use crate::cli::ClockChoice;
use crate::state::{StartError, State};
use casement_core::time::Mono;
use smithay::reexports::calloop::LoopHandle;
use std::time::Instant;

/// What `Mono` is in this run. (The sketch held `VirtualClock` by value; the calloop source
/// shares it, so it is a shared handle: FINDINGS.)
#[derive(Debug, Clone)]
pub enum ClockSource {
    /// The monotonic wall clock, from this epoch.
    Monotonic(Instant),
    /// A virtual clock the test kit advances.
    Virtual(SharedClock),
    /// A virtual clock `casementctl debug clock` steps.
    Stepped(SharedClock),
}

impl ClockSource {
    /// The clock `choice` asks for, its timer source inserted into `event_loop`.
    pub fn start(
        choice: ClockChoice,
        event_loop: &LoopHandle<'static, State>,
    ) -> Result<ClockSource, StartError> {
        let _ = (choice, event_loop);
        todo!("P0-B-loop: ClockSource::start")
    }

    /// Now, on this clock.
    pub fn now(&self) -> Mono {
        todo!("P0-B-loop: ClockSource::now")
    }
}
