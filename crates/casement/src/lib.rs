//! casement's composition root: the one [`State`] smithay dispatches into, its handlers
//! (`impl <Smithay>Handler for State`, one file per protocol, C28), the core loop
//! (`Model::step` → arrange → retarget → frames), the one place effects happen ([`exec`]),
//! the only clock readers ([`time`]), the stored settings document ([`config`]) and the
//! command line ([`cli`]).
//!
//! The handler recipe (CONVENTIONS addendum 4): a smithay callback in `handlers/<p>.rs`
//! converts its arguments and calls `casement_wayland::<p>::on_<request>` → `Vec<Event>` →
//! [`core_loop::feed`] → `Model::step` → `Vec<Effect>` → [`exec::exec`] →
//! `casement_wayland::<p>::apply_<effect>`, the backend or the renderer.

pub mod cli;
pub mod clients;
pub mod config;
pub mod core_loop;
mod delegates;
pub mod display;
pub mod exec;
mod handlers;
#[cfg(any(feature = "testing", feature = "debug-inject"))]
pub mod jail_check;
pub mod log;
#[cfg(feature = "testing")]
pub mod relax;
mod state;
pub mod time;

pub use state::{AgentState, CoreState, IpcState, RenderState, StartError, State};
