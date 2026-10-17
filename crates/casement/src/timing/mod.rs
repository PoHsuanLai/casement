//! The timing lines: four plain `tracing` events at INFO, target `timing`, that say how long the
//! desktop itself takes (a window from its client's connection to its first frame, a change of
//! key window from the key press to the frame, the first X11 program, the login). The contract
//! is `~/rs-wt/workflow-spec.md`; the desktop's `workflow/` scripts read the lines.
//!
//! ```text
//! window-ready app_id=<id> pid=<pid> connected_at=<ns> first_commit_at=<ns> first_frame_at=<ns> settled_at=<ns>
//! key-window app_id=<id> cause=<keyboard|pointer|request|other> input_at=<ns|0> focused_at=<ns> presented_at=<ns>
//! xwayland-ready requested_at=<ns> ready_at=<ns>
//! session-ready started_at=<ns> shell_connected_at=<ns> shell_ready_at=<ns>
//! ```
//!
//! Every time is `CLOCK_MONOTONIC` in nanoseconds ([`Stamp`]). The session clock's `Mono` counts
//! from an epoch whose `CLOCK_MONOTONIC` reading is taken once at start
//! (`ClockSource::monotonic_epoch_ns`), so a stamp is that epoch plus a `Mono`: no second clock
//! is read to make one. The lines carry app ids and process ids only, never a title, a file name
//! or input content, and are always on: a handful per launch, one per change of key window.
//!
//! The records are pure and table-tested ([`FirstShow`], [`KeyChange`], [`SessionStage`],
//! [`XwaylandStage`], the `line` of each finished record); [`Timing`] keeps them in ordered maps in
//! `State`; [`Sink`] is the only effect. The `hear` functions are the glue the loop calls at each
//! moment, and each returns at once when nothing is waiting (a frame costs a few comparisons).

mod book;
mod first_show;
mod hear;
mod key_window;
mod keys;
mod session;
mod session_ready;
mod sink;
mod stamp;
mod token;
mod trigger;
mod window_ready;
mod xwayland_ready;

pub use book::Timing;
pub use first_show::{Awaiting, FirstShow, Opened};
pub use key_window::{KeyCause, KeyChange, KeyWindow};
pub use session_ready::{SessionReady, SessionStage, ShellSurface};
pub use sink::Sink;
pub use stamp::Stamp;
pub use token::Token;
pub use trigger::{DIRECT_WINDOW, FeedKind, Press, PressKind, REQUEST_WINDOW, cause_of};
pub use window_ready::WindowReady;
pub use xwayland_ready::{XwaylandReady, XwaylandStage};

pub(crate) use hear::{
    accepted_at, connected, emphasis, enter, flipped, hear, leave, satellite_started, unchanged,
};
