//! The fuzz ops.

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
    /// The same request again and again (a region of 10k rectangles, a storm of commits).
    Repeated {
        /// How many times, at most 10 000.
        times: u16,
        /// What is sent each time.
        request: Box<FuzzRequest>,
    },
}

/// The most a `Repeated` request sends.
pub const MAX_REPEAT: u16 = 10_000;
