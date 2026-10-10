//! Bindings that call sill (A4, P1-Z; filled by P1-I1): the `ShellCalls` seam and its D-Bus
//! implementation. The composition root names the call (a `DbusCall` from a binding) and never
//! zbus; this crate, the one D-Bus crate, makes it. `casement-testkit` exports `FakeShellCalls`,
//! which records the calls, so a test asserts that a binding called the method within one
//! frame.
//!
//! A call is fire-and-forget from the loop's point of view: [`ShellCalls::call`] hands it to
//! the bus and returns; a call that fails later is reported by [`ShellCalls::failures`], which
//! the loop logs.
//!
//! ```no_run
//! use casement_core::input::DbusCall;
//! use casement_ipc::shell_calls::{ShellCalls, ZbusShellCalls};
//!
//! let mut calls = ZbusShellCalls::lazy();
//! let toggle = DbusCall {
//!     service: "org.quire.Sill".to_owned(),
//!     path: "/org/quire/Sill".to_owned(),
//!     interface: "org.quire.Sill1".to_owned(),
//!     method: "ToggleLauncher".to_owned(),
//!     args: Vec::new(),
//! };
//! calls.call(&toggle).expect("sent");
//! ```

use casement_core::input::DbusCall;

/// Why a call could not be made.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ShellCallError {
    /// A name, path or interface in the call is not valid D-Bus.
    #[error("bad {what} in a binding's call: {text}")]
    BadName {
        /// Which part (`"service"`, `"path"`, `"interface"`, `"method"`).
        what: &'static str,
        /// The text as given.
        text: String,
    },
    /// The session bus refused.
    #[error("bus: {0}")]
    Bus(#[from] zbus::Error),
}

/// A call that was sent and then failed.
#[derive(Debug)]
#[non_exhaustive]
pub struct FailedCall {
    /// The call.
    pub call: DbusCall,
    /// Why it failed.
    pub why: ShellCallError,
}

/// The session-bus calls a binding makes.
pub trait ShellCalls: std::fmt::Debug {
    /// Sends `call` (string arguments, no reply awaited).
    fn call(&mut self, call: &DbusCall) -> Result<(), ShellCallError>;

    /// The calls that failed since the last time this was asked.
    fn failures(&mut self) -> Vec<FailedCall>;
}

/// The real implementation over zbus on the session bus, connected on first use.
#[derive(Debug, Default)]
pub struct ZbusShellCalls {
    failed: Vec<FailedCall>,
}

impl ZbusShellCalls {
    /// A caller that connects to the session bus when the first call is made.
    pub fn lazy() -> ZbusShellCalls {
        ZbusShellCalls::default()
    }
}

impl ShellCalls for ZbusShellCalls {
    fn call(&mut self, call: &DbusCall) -> Result<(), ShellCallError> {
        let _ = call;
        todo!("P1-I1: ZbusShellCalls::call")
    }

    fn failures(&mut self) -> Vec<FailedCall> {
        std::mem::take(&mut self.failed)
    }
}
