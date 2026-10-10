//! casement's agent edge (C10, C20, B14): the only crate that hands out privileged Wayland
//! connections, and from P4 the agent seat, EIS, masked capture, the trusted surface and the
//! kill chord. At P2-Z the hand-out's types and signatures; bodies at P2 (P2-Q2).

mod socket;

pub use socket::{CallerUnit, HandOut, Handed, RefuseWhy, hand_out, role_for};
