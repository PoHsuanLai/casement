//! Where vblanks come from: the session clock's `VblankClock` per output, or a fixed-rate
//! timer for the winit backend. (Real page flips are P1's `ScanoutSurface`.)

use casement_core::time::{Millihertz, VblankClock};

/// An output's vblank source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VblankSource {
    /// Vblanks on the session clock (virtual in tests), at the output's phase.
    Virtual(VblankClock),
    /// A timer at this rate (winit; the timer lives in `casement/src/time/`).
    Timer(Millihertz),
}
