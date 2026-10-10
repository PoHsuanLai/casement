//! Registering the Spaces casement mints with the account service (`org.quire.Spaces1.Register`).
//!
//! casement mints a Space's id (the ext-workspace id, kept in `spaces.toml`); the account service
//! learns of it here, so an app's own Space can link to it. [`SpaceRegistry`] is the seam:
//! [`SpaceRegistry::register`] hands a batch to a background thread and returns at once, and
//! [`SpaceRegistry::failures`] tells what the service answered later. The real implementation,
//! [`ZbusSpaceRegistry`], calls `porter_dbus::SpacesProxy::register` on the bus at an address it
//! is given (the library reads no environment); `casement-testkit` exports `FakeSpaceRegistry`.
//!
//! The service registers a known id again without changing it, so the caller registers every
//! Space at each start. A refusal means casement is not yet the unit the service gives the
//! compositor role to ([`RegisterFault::Refused`]); the caller stops asking after the first.
//! The service allows ten new Spaces a minute per caller: the thread asks at most [`PACE`]
//! times, then waits [`PACE_WINDOW`] before the next, so a long batch finishes later and never
//! trips the limit.
//!
//! ```
//! use casement_ipc::space_registry::{NoSpaceRegistry, SpaceRegistration, SpaceRegistry};
//! use porter_core::SpaceId;
//!
//! let mut registry = NoSpaceRegistry;
//! let id = SpaceId::parse("s-0123").expect("an id");
//! registry.register(vec![SpaceRegistration::new(id, "Desktop 1".to_owned())]).expect("queued");
//! assert!(registry.failures().is_empty());
//! ```

mod pace;
mod worker;

pub use pace::{PACE, PACE_WINDOW, Pace};
use porter_core::SpaceId;
use worker::Worker;

/// The look sent with every registration: empty is the default look ("no look chosen").
pub const NO_LOOK: &str = "";

/// The most registrations waiting for the background thread; more are refused with
/// [`RegistryError::QueueFull`] and asked again at the next change.
pub const QUEUE_LIMIT: usize = 64;

/// One Space to register.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SpaceRegistration {
    /// Its id, as minted.
    pub id: SpaceId,
    /// Its plain name ("Desktop 1").
    pub name: String,
}

impl SpaceRegistration {
    /// The registration of `id` under `name`.
    pub fn new(id: SpaceId, name: String) -> SpaceRegistration {
        SpaceRegistration { id, name }
    }
}

/// Why a registration did not go through.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RegisterFault {
    /// The service does not give this process the compositor role (it is not the unit yet):
    /// asking again changes nothing.
    Refused,
    /// The service is not there yet, or asked for less: ask again at the next change.
    Later,
    /// Anything else, with the service's words.
    Failed(String),
}

/// A registration that failed after it was queued.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct RegisterFailure {
    /// The Space.
    pub id: SpaceId,
    /// Why.
    pub fault: RegisterFault,
}

/// Why a batch could not be queued.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RegistryError {
    /// The thread that makes the calls could not be started or has stopped.
    #[error("the Spaces registration thread: {0}")]
    Thread(std::io::Error),
    /// Too many registrations are waiting.
    #[error("too many Spaces are waiting to be registered")]
    QueueFull,
}

/// Where the Spaces casement mints are registered.
pub trait SpaceRegistry: std::fmt::Debug {
    /// Queues `spaces` and returns; the answers come back through [`SpaceRegistry::failures`].
    fn register(&mut self, spaces: Vec<SpaceRegistration>) -> Result<(), RegistryError>;

    /// The registrations that failed since the last time this was asked.
    fn failures(&mut self) -> Vec<RegisterFailure>;
}

/// Registers nothing (no bus is configured: a headless run, a fixture).
#[derive(Debug, Clone, Copy, Default)]
pub struct NoSpaceRegistry;

impl SpaceRegistry for NoSpaceRegistry {
    fn register(&mut self, spaces: Vec<SpaceRegistration>) -> Result<(), RegistryError> {
        let _ = spaces;
        Ok(())
    }

    fn failures(&mut self) -> Vec<RegisterFailure> {
        Vec::new()
    }
}

/// The real implementation: `Spaces1.Register` over zbus on the bus at `address`, from a thread
/// started with the first batch.
#[derive(Debug)]
pub struct ZbusSpaceRegistry {
    address: String,
    worker: Option<Worker>,
}

impl ZbusSpaceRegistry {
    /// A registry calling the bus at `address` (the session bus).
    pub fn at(address: String) -> ZbusSpaceRegistry {
        ZbusSpaceRegistry {
            address,
            worker: None,
        }
    }
}

impl SpaceRegistry for ZbusSpaceRegistry {
    fn register(&mut self, spaces: Vec<SpaceRegistration>) -> Result<(), RegistryError> {
        if self.worker.is_none() {
            let worker = Worker::start(self.address.clone()).map_err(RegistryError::Thread)?;
            self.worker = Some(worker);
        }
        let outcome = match &self.worker {
            Some(worker) => spaces.into_iter().try_for_each(|space| worker.send(space)),
            None => Ok(()),
        };
        if matches!(outcome, Err(RegistryError::Thread(_))) {
            self.worker = None;
        }
        outcome
    }

    fn failures(&mut self) -> Vec<RegisterFailure> {
        self.worker
            .as_ref()
            .map(Worker::failures)
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests;
