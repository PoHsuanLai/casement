//! The fake account service: `FakeSpaceRegistry` records the Spaces casement registers and
//! answers failures a test scripts. A clone shares the record.

use casement_ipc::space_registry::{
    RegisterFailure, RegisterFault, RegistryError, SpaceRegistration, SpaceRegistry,
};
use std::sync::{Arc, Mutex, PoisonError};

#[derive(Debug, Default)]
struct Inner {
    batches: Vec<Vec<SpaceRegistration>>,
    scripted: Vec<RegisterFault>,
    failures: Vec<RegisterFailure>,
}

/// A recorder standing in for `org.quire.Spaces1`.
#[derive(Debug, Clone, Default)]
pub struct FakeSpaceRegistry {
    inner: Arc<Mutex<Inner>>,
}

impl FakeSpaceRegistry {
    /// A registry that accepts everything.
    pub fn new() -> FakeSpaceRegistry {
        FakeSpaceRegistry::default()
    }

    /// The next registered Space fails with `fault` (one fault per Space, in the order scripted).
    pub fn fail_next(&self, fault: RegisterFault) {
        self.lock().scripted.push(fault);
    }

    /// The batches registered so far, in order.
    pub fn batches(&self) -> Vec<Vec<SpaceRegistration>> {
        self.lock().batches.clone()
    }

    /// The ids registered so far, in order, across batches.
    pub fn ids(&self) -> Vec<String> {
        self.lock()
            .batches
            .iter()
            .flatten()
            .map(|space| space.id.as_str().to_owned())
            .collect()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl SpaceRegistry for FakeSpaceRegistry {
    fn register(&mut self, spaces: Vec<SpaceRegistration>) -> Result<(), RegistryError> {
        let mut inner = self.lock();
        for space in &spaces {
            if !inner.scripted.is_empty() {
                let fault = inner.scripted.remove(0);
                let id = space.id.clone();
                inner.failures.push(RegisterFailure { id, fault });
            }
        }
        inner.batches.push(spaces);
        Ok(())
    }

    fn failures(&mut self) -> Vec<RegisterFailure> {
        std::mem::take(&mut self.lock().failures)
    }
}
