//! The frame scheduler's maths (P1-Z; filled by P1-D3): pure functions of a vblank clock, a
//! render-cost history and a margin. The backend's `ScanoutSurface` and the headless vblank
//! source feed it the same numbers, so 36,000 virtual idle frames and a real tty run the same
//! decisions.
//!
//! ```no_run
//! use casement_core::frame::{RenderHistory, next_deadline, predict};
//! use casement_core::time::{Millihertz, Mono, VblankClock};
//! use std::time::Duration;
//!
//! let clock = VblankClock::new(Millihertz(60_000), Mono(Duration::ZERO));
//! let history = RenderHistory::empty();
//! let deadline = next_deadline(
//!     &clock,
//!     predict(&history),
//!     Mono(Duration::from_millis(3)),
//!     Duration::from_millis(1),
//! );
//! assert!(deadline.render_at <= deadline.vblank);
//! ```

mod cursor_plane;
mod outcome;
mod predict;
mod schedule;

pub use cursor_plane::{CursorPlaneCaps, CursorPlaneChoice, cursor_plane_for};
pub use outcome::{FrameOutcome, judge};
pub use predict::{RenderHistory, predict};
pub use schedule::{Deadline, next_deadline};
