//! Typed fuzzing (11 §3.5): sequences of requests from clients of every role on valid,
//! destroyed, foreign and null objects, interleaved with clock advances, checked by five
//! oracles (no panic, invariants hold after every step, I7 holds, a protocol error disconnects
//! only its client, the transcript stays well-formed). Bodies: P0-T-client.

mod ops;

pub use ops::{FuzzOp, FuzzRequest, ObjectRef, run_ops};
