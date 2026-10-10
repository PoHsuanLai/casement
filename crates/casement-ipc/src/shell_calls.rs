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
//! //! use casement_core::input::DbusCall;
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

mod worker;

use casement_core::input::DbusCall;
use worker::{MOST_ARGS, Worker};
use zbus::names::{BusName, InterfaceName, MemberName};
use zbus::zvariant::ObjectPath;

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
    /// The call carries more string arguments than a binding may pass.
    #[error("a binding's call passes at most {most} arguments, this one has {given}")]
    TooManyArgs {
        /// The most allowed.
        most: usize,
        /// How many the call has.
        given: usize,
    },
    /// The thread that makes the calls could not be started or has stopped.
    #[error("the shell-call thread: {0}")]
    Thread(std::io::Error),
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
    worker: Option<Worker>,
}

impl ZbusShellCalls {
    /// A caller that connects to the session bus when the first call is made.
    pub fn lazy() -> ZbusShellCalls {
        ZbusShellCalls::default()
    }
}

/// Checks the names in `call` the way the bus will, before anything is queued.
fn checked(call: &DbusCall) -> Result<(), ShellCallError> {
    let bad = |what: &'static str, text: &str| ShellCallError::BadName {
        what,
        text: text.to_owned(),
    };
    BusName::try_from(call.service.as_str()).map_err(|_| bad("service", &call.service))?;
    ObjectPath::try_from(call.path.as_str()).map_err(|_| bad("path", &call.path))?;
    InterfaceName::try_from(call.interface.as_str())
        .map_err(|_| bad("interface", &call.interface))?;
    MemberName::try_from(call.method.as_str()).map_err(|_| bad("method", &call.method))?;
    match call.args.len() {
        given if given > MOST_ARGS => Err(ShellCallError::TooManyArgs {
            most: MOST_ARGS,
            given,
        }),
        _ => Ok(()),
    }
}

impl ShellCalls for ZbusShellCalls {
    fn call(&mut self, call: &DbusCall) -> Result<(), ShellCallError> {
        checked(call)?;
        let worker = match self.worker.take() {
            Some(worker) => worker,
            None => Worker::start().map_err(ShellCallError::Thread)?,
        };
        match worker.send(call.clone()) {
            Ok(()) => {
                self.worker = Some(worker);
                Ok(())
            }
            // The thread is gone: the next call starts a new one.
            Err(_) => Err(ShellCallError::Thread(std::io::Error::other(
                "the thread has stopped",
            ))),
        }
    }

    fn failures(&mut self) -> Vec<FailedCall>;
}

/// The real implementation over zbus on the session bus, connected on first use.
#[derive(Debug, Default)]
pub struct ZbusShellCalls {
    worker: Option<Worker>,
}

impl ZbusShellCalls {
    /// A caller that connects to the session bus when the first call is made.
    pub fn lazy() -> ZbusShellCalls {
        ZbusShellCalls::default()
    }
}

/// Checks the names in `call` the way the bus will, before anything is queued.
fn checked(call: &DbusCall) -> Result<(), ShellCallError> {
    let bad = |what: &'static str, text: &str| ShellCallError::BadName {
        what,
        text: text.to_owned(),
    };
    BusName::try_from(call.service.as_str()).map_err(|_| bad("service", &call.service))?;
    ObjectPath::try_from(call.path.as_str()).map_err(|_| bad("path", &call.path))?;
    InterfaceName::try_from(call.interface.as_str())
        .map_err(|_| bad("interface", &call.interface))?;
    MemberName::try_from(call.method.as_str()).map_err(|_| bad("method", &call.method))?;
    match call.args.len() {
        given if given > MOST_ARGS => Err(ShellCallError::TooManyArgs {
            most: MOST_ARGS,
            given,
        }),
        _ => Ok(()),
    }
}

impl ShellCalls for ZbusShellCalls {
    fn call(&mut self, call: &DbusCall) -> Result<(), ShellCallError> {
        checked(call)?;
        if self.worker.is_none() {
            self.worker = Some(Worker::start().map_err(ShellCallError::Thread)?);
        }
        let Some(worker) = &self.worker else {
            return Ok(());
        };
        worker.send(call.clone()).map_err(|_| {
            self.worker = None;
            ShellCallError::Thread(std::io::Error::other("the thread has stopped"))
        })
    }

    fn failures(&mut self) -> Vec<FailedCall> {
        self.worker
            .as_ref()
            .map(Worker::failures)
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(service: &str, path: &str, interface: &str, method: &str) -> DbusCall {
        DbusCall {
            service: service.to_owned(),
            path: path.to_owned(),
            interface: interface.to_owned(),
            method: method.to_owned(),
            args: Vec::new(),
        }
    }

    #[test]
    fn a_name_that_is_not_d_bus_is_refused_before_anything_is_queued() {
        let mut calls = ZbusShellCalls::lazy();
        for (case, bad, what) in [
            ("service", call("nodots", "/a", "org.a.B", "M"), "service"),
            ("path", call("org.a.B", "a", "org.a.B", "M"), "path"),
            ("interface", call("org.a.B", "/a", "x", "M"), "interface"),
            ("method", call("org.a.B", "/a", "org.a.B", "1 2"), "method"),
        ] {
            match calls.call(&bad) {
                Err(ShellCallError::BadName { what: got, .. }) => assert_eq!(got, what, "{case}"),
                other => panic!("{case}: {other:?}"),
            }
        }
        assert!(calls.worker.is_none(), "nothing started a thread");
    }

    #[test]
    fn too_many_arguments_are_refused() {
        let mut calls = ZbusShellCalls::lazy();
        let mut many = call("org.a.B", "/a", "org.a.B", "M");
        many.args = vec!["x".to_owned(); MOST_ARGS + 1];
        assert!(matches!(
            calls.call(&many),
            Err(ShellCallError::TooManyArgs { .. })
        ));
    }
}
