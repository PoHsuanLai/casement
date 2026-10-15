//! Apps: who owns windows, by proven or claimed identity (03 §1.1, §2.9).

#[allow(
    clippy::module_inception,
    reason = "the freeze plan names the record file after its concept"
)]
mod app;
mod key;
mod menu;
mod quit;

pub use app::{App, AppLife, AppVisibility, PidSet, QuitPhase};
pub use key::AppKey;
pub use menu::MenuSource;
pub use quit::{
    Answer, ForceQuitRow, KILL_GRACE, PING_DEADLINE, PING_GAP, Pinging, QUIT_GRACE, StopSignal,
    Termination, force_quit_rows, ping_plan,
};
