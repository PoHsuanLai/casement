//! Keeping the account service told of the Spaces casement mints.
//!
//! [`SpaceSync`] sits between the model and the [`SpaceRegistry`] seam: [`SpaceSync::sync`] sends
//! the desktops it has not sent yet (all of them at the first call after the Spaces are loaded,
//! one at a time as they are made), and [`SpaceSync::poll`] reads what came back. A refusal (casement
//! is not yet the unit the service gives the compositor role to) is logged once and ends the
//! asking for the run; a service that is not there, or a rate limit, takes the Space back out
//! of the sent set so the next change asks again. Nothing here waits: the seam queues.

use casement_core::space::{SpaceBook, SpaceContent};
use casement_ipc::space_registry::{
    NoSpaceRegistry, RegisterFault, SpaceRegistration, SpaceRegistry,
};
use porter_core::SpaceId;
use std::collections::BTreeSet;

/// Whether the service still listens to us.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Standing {
    Asking,
    Refused,
}

/// The Spaces sent, and the registry they were sent to.
#[derive(Debug)]
pub struct SpaceSync {
    registry: Box<dyn SpaceRegistry>,
    sent: BTreeSet<SpaceId>,
    standing: Standing,
}

impl SpaceSync {
    /// Registers through `registry`.
    pub fn new(registry: Box<dyn SpaceRegistry>) -> SpaceSync {
        SpaceSync {
            registry,
            sent: BTreeSet::new(),
            standing: Standing::Asking,
        }
    }

    /// Registers nothing.
    pub fn none() -> SpaceSync {
        SpaceSync::new(Box::new(NoSpaceRegistry))
    }

    /// Sends every desktop in `book` that was not sent yet, in one batch.
    pub fn sync(&mut self, book: &SpaceBook) {
        self.poll();
        if self.standing == Standing::Refused {
            return;
        }
        let fresh: Vec<SpaceRegistration> = book
            .iter()
            .filter(|space| matches!(space.content, SpaceContent::Desktop))
            .filter(|space| !self.sent.contains(&space.id))
            .map(|space| SpaceRegistration::new(space.id.clone(), space.plain_name()))
            .collect();
        if fresh.is_empty() {
            return;
        }
        let ids: Vec<SpaceId> = fresh.iter().map(|r| r.id.clone()).collect();
        self.sent.extend(ids.iter().cloned());
        if let Err(error) = self.registry.register(fresh) {
            tracing::warn!(%error, "Spaces could not be queued for registration; they are asked again at the next change");
            for id in &ids {
                self.sent.remove(id);
            }
        }
    }

    /// Reads the answers that came back since the last call.
    pub fn poll(&mut self) {
        for failure in self.registry.failures() {
            match failure.fault {
                RegisterFault::Refused => {
                    if self.standing == Standing::Asking {
                        tracing::warn!(
                            "the account service does not take this compositor's Spaces yet (it is not running as its service); they stay local"
                        );
                    }
                    self.standing = Standing::Refused;
                }
                RegisterFault::Later => {
                    tracing::debug!(space = %failure.id, "the account service was not ready for a Space; asking again at the next change");
                    self.sent.remove(&failure.id);
                }
                RegisterFault::Failed(why) => {
                    tracing::warn!(space = %failure.id, %why, "a Space was not registered");
                }
                _ => {}
            }
        }
    }
}
