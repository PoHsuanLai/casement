//! What a failed oracle looks like: one variant per way the five oracles (`oracles.rs`) can
//! fire, so a harness matches on the failure and the message is only the rendering.

use super::ops::FuzzOp;
use super::oracles::Snapshot;
use crate::fixture::ClientId;
use casement_core::InvariantBreak;
use casement_core::policy::ConnectionRole;
use casement_testclient::{ClientError, ClientProtocolError};
use std::fmt;

/// Where the compositor panicked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PanicSite {
    /// While pumping the parties until quiet.
    Pump,
    /// While the clock moved.
    Advance,
    /// While a request was sent.
    Request,
}

impl fmt::Display for PanicSite {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            PanicSite::Pump => "while pumping",
            PanicSite::Advance => "in advance",
            PanicSite::Request => "on a request",
        })
    }
}

/// Why a fuzz run failed: the oracle that fired, and for [`FuzzFailure::AtOp`] the op after
/// which it did.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum FuzzFailure {
    /// Oracle 1: the compositor panicked.
    #[error("oracle 1: the compositor panicked {0}")]
    Panicked(PanicSite),
    /// Oracle 2: a client got a protocol error its interface does not document.
    #[error("oracle 2: client {client:?} got an undocumented protocol error: {error:?}")]
    UndocumentedError {
        /// The client.
        client: ClientId,
        /// The error it got.
        error: ClientProtocolError,
    },
    /// Oracle 3: the bystander could not even ask for a sync.
    #[error("oracle 3: the bystander could not sync: {0}")]
    BystanderSync(#[from] ClientError),
    /// Oracle 3: the bystander got a protocol error.
    #[error("oracle 3: the bystander got a protocol error: {0:?}")]
    BystanderError(ClientProtocolError),
    /// Oracle 3: the bystander's roundtrip did not complete.
    #[error("oracle 3: the bystander's roundtrip did not complete")]
    BystanderStalled,
    /// Oracle 3: the bystander's state changed.
    #[error("oracle 3: the bystander's state changed: {before:?} -> {after:?}")]
    BystanderChanged {
        /// How it looked before.
        before: Box<Snapshot>,
        /// How it looks now.
        after: Box<Snapshot>,
    },
    /// Oracle 4: the model's invariants do not hold.
    #[error("oracle 4: an invariant broke: {0:?}")]
    InvariantBroke(InvariantBreak),
    /// Oracle 5: a privileged global was offered to a connection that may not have it.
    #[error("oracle 5: {role:?} was offered the privileged global {interface}")]
    PrivilegedOffered {
        /// The connection's role.
        role: ConnectionRole,
        /// The global's interface.
        interface: String,
    },
    /// Oracle 5: a global was offered to a role that may not see it.
    #[error("oracle 5: {role:?} was offered {interface}, which its role may not see")]
    GlobalNotForRole {
        /// The connection's role.
        role: ConnectionRole,
        /// The global's interface.
        interface: String,
    },
    /// An oracle fired after this op.
    #[error("op {step} ({op:?}): {failure}")]
    AtOp {
        /// The op's place in the sequence.
        step: usize,
        /// The op.
        op: Box<FuzzOp>,
        /// The oracle that fired.
        failure: Box<FuzzFailure>,
    },
}
