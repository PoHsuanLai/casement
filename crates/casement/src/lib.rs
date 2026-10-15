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
//!
//! # Embedding the compositor
//!
//! [`run`] is the whole startup: it builds the event loop and the Wayland display, makes the
//! [`State`] with [`State::new`], listens for clients and drives the loop until it is asked to
//! stop, so a second binary or a harness calls it with a [`RunConfig`] and gets what the
//! `casement` binary gets. A host that wants its own loop does what `run` does: make a
//! `calloop` event loop and a `Display<State>`, call `State::new(config, &mut display, &handle)`,
//! attach the display ([`display::attach_display`]) and call [`core_loop::turn`] once per
//! loop turn. `State` is the one type smithay dispatches into, so the host builds with the union
//! of smithay features this crate lists in its manifest (`wayland_frontend`, `desktop`, the
//! renderers, the backends it starts and `use_system_lib`); the pure crates under it
//! (`casement-core`, `-layout`, `-motion`) need none of them.
//!
//! ```
//! use casement::RunConfig;
//! use casement::cli::{self, Command};
//!
//! let Ok(Command::Run(start)) = cli::parse_args(&[]) else {
//!     panic!("no arguments start the compositor");
//! };
//! let config = RunConfig::new(*start);
//! // Compiled, not called: `run` blocks until the compositor is asked to stop.
//! let _serve = move || casement::run(config);
//! ```

pub mod appearance;
pub mod cli;
pub mod clients;
pub mod config;
pub mod connect;
pub mod core_loop;
#[cfg(feature = "debug-inject")]
mod debug;
mod delegates;
pub mod display;
pub mod exec;
mod handlers;
pub mod icons;
#[cfg(any(feature = "testing", feature = "debug-inject"))]
pub mod jail_check;
pub mod keymap;
pub mod log;
pub mod notify;
#[cfg(feature = "testing")]
pub mod relax;
mod run;
pub mod seams;
pub mod settings_live;
pub mod space_store;
pub mod space_sync;
pub mod spawn;
mod state;
pub mod suspend;
pub mod test_mode;
pub mod time;
pub mod xwayland;

pub use run::{CompositorBus, DebugBus, RunConfig, RunError, run};
pub use state::{AgentState, CoreState, IpcState, RenderState, StartError, State};
