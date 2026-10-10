//! Typed fuzzing (11 §3.5): sequences of requests from clients of every role on valid,
//! destroyed, foreign and null objects, interleaved with clock advances, checked after every
//! op by the five oracles (`oracles.rs`): the compositor is alive; a protocol error to the
//! offender is a documented one; a healthy bystander still roundtrips and looks the same; the
//! model's invariants hold; privileged globals stay off connections that may not see them.

mod failure;
mod host;
mod ops;
mod oracles;
mod prime;
mod run;
#[cfg(test)]
mod scripted;
pub mod strategy;

pub use failure::{FuzzFailure, PanicSite};
pub use host::FuzzHost;
pub use ops::{FuzzOp, FuzzRequest, MAX_REPEAT, ObjectRef};
pub use oracles::Snapshot;
pub use run::{run_on, run_ops};
