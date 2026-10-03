//! A test client's connection.

use crate::globals::GlobalList;
use crate::pump::Pumped;
use crate::transcript::Transcript;
use std::os::fd::OwnedFd;

/// One test client: its connection, its event queue, the globals it saw and its transcript.
#[derive(Debug)]
pub struct TestClient {
    #[expect(dead_code, reason = "P0-T-client: the connection")]
    connection: wayland_client::Connection,
    globals: GlobalList,
    transcript: Transcript,
}

/// Why a test client could not connect or talk.
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    /// The connection failed.
    #[error("connect: {0}")]
    Connect(String),
    /// A roundtrip or dispatch failed (the compositor sent a protocol error or went away).
    #[error("dispatch: {0}")]
    Dispatch(String),
    /// A global the helper needs is not advertised.
    #[error("missing global {0}")]
    MissingGlobal(String),
}

impl TestClient {
    /// A client over the client end of a fixture's socket pair (non-blocking; one bounded
    /// roundtrip for the registry).
    pub fn over(fd: OwnedFd) -> Result<TestClient, ClientError> {
        let _ = fd;
        todo!("P0-T-client: TestClient::over")
    }

    /// A client of `$WAYLAND_DISPLAY` (the jailed binary), with a bounded roundtrip.
    pub fn connect_env() -> Result<TestClient, ClientError> {
        todo!("P0-T-client: TestClient::connect_env")
    }

    /// Reads what is there without blocking and dispatches it; says whether anything moved.
    pub fn pump(&mut self) -> Pumped {
        todo!("P0-T-client: TestClient::pump")
    }

    /// The globals the registry advertised.
    pub fn globals(&self) -> &GlobalList {
        &self.globals
    }

    /// Every event this client received, in order.
    pub fn transcript(&self) -> &Transcript {
        &self.transcript
    }
}
