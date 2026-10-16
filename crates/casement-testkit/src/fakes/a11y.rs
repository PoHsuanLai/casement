//! The accessibility-root checker as a test sees it: it records the claims casement queued and
//! never asks a bus. The test then says what the check came to
//! ([`Fixture::a11y_verdict`](crate::Fixture::a11y_verdict)).

use casement::a11y::A11yChecks;
use casement_ipc::a11y::{A11yAsk, A11yFault};
use std::sync::{Arc, Mutex, PoisonError};

/// The claims queued for checking. A clone shares the list.
#[derive(Debug, Clone, Default)]
pub struct FakeA11y {
    asked: Arc<Mutex<Vec<A11yAsk>>>,
}

impl FakeA11y {
    /// A checker with nothing queued.
    pub fn new() -> FakeA11y {
        FakeA11y::default()
    }

    /// Every claim queued so far, in order.
    pub fn asked(&self) -> Vec<A11yAsk> {
        self.asked
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl A11yChecks for FakeA11y {
    fn check(&self, ask: A11yAsk) -> Result<(), A11yFault> {
        self.asked
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(ask);
        Ok(())
    }
}
