//! The fuzz ops.

use crate::fixture::Fixture;
use casement_core::policy::ConnectionRole;

/// One step of a fuzz run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FuzzOp {
    /// A request from a client of `role`.
    Request {
        /// The client's role.
        role: ConnectionRole,
        /// The object it is sent on.
        object: ObjectRef,
        /// The request.
        request: FuzzRequest,
    },
    /// Advance the clock this many milliseconds.
    Advance(u32),
}

/// Which object a fuzzed request names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObjectRef {
    /// A live object of the sender (by creation order).
    Valid(u32),
    /// An object the sender destroyed.
    Destroyed(u32),
    /// An object of another client.
    Foreign(u32),
    /// No object.
    Null,
}

/// A request, by protocol and name, with fuzzed integer arguments (one variant per private
/// request from P2: CONVENTIONS addendum 11).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FuzzRequest {
    /// A request of a standard protocol, by interface and opcode.
    Wire {
        /// The interface.
        interface: String,
        /// The opcode.
        opcode: u16,
        /// Integer arguments.
        args: Vec<i32>,
    },
}

/// Runs `ops` against `fixture`, checking the five oracles after each; the first failure's
/// description.
pub fn run_ops(fixture: &mut Fixture, ops: &[FuzzOp]) -> Result<(), String> {
    let _ = (fixture, ops);
    todo!("P0-T-client: run_ops")
}
