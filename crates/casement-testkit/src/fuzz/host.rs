//! What the fuzz runner needs from a compositor: the fixture, or a stand-in in the runner's own
//! tests.

use crate::fixture::{ClientId, Fixture};
use casement_core::InvariantBreak;
use casement_core::policy::ConnectionRole;
use casement_testclient::{Pumped, TestClient};
use std::time::Duration;

/// A compositor the runner can drive.
pub trait FuzzHost {
    /// A new client of `role`, already pumped.
    fn connect(&mut self, role: ConnectionRole) -> ClientId;
    /// A client.
    fn client(&mut self, id: ClientId) -> &mut TestClient;
    /// Pumps every party until quiet.
    fn pump(&mut self) -> Pumped;
    /// Moves the clock.
    fn advance(&mut self, by: Duration);
    /// The model's invariants (oracle 4).
    fn invariants(&self) -> Result<(), InvariantBreak>;
}

impl FuzzHost for Fixture {
    fn connect(&mut self, role: ConnectionRole) -> ClientId {
        Fixture::connect(self, role)
    }

    fn client(&mut self, id: ClientId) -> &mut TestClient {
        Fixture::client(self, id)
    }

    fn pump(&mut self) -> Pumped {
        Fixture::pump(self)
    }

    fn advance(&mut self, by: Duration) {
        Fixture::advance(self, by);
    }

    fn invariants(&self) -> Result<(), InvariantBreak> {
        self.model().check_invariants()
    }
}
