//! casement's agent edge (C10, C20, B14): the only crate that hands out privileged Wayland
//! connections, and from P4 the agent seat, EIS, masked capture, the trusted surface and the
//! kill chord. At P2-Z the hand-out's types and signatures; bodies at P2 (P2-Q2).

mod eis;
mod eis_keymap;
mod remote;
mod socket;

pub use eis::{
    RemoteEisSource, RemoteEvent, RemoteGrant, RemoteInput, RemoteKind, RemoteKinds,
    agent_may_lease, open as open_remote_eis, portal_may_connect,
};
pub use eis_keymap::{KeymapFile, KeymapText};
pub use remote::{
    MAX_HELD, MAX_REMOTE_SLOTS, RemoteFor, RemoteHandle, RemoteSlot, RemoteSlots,
    device_id as remote_device_id,
};

pub use peephole_core::{AppKey, RoleName};
pub use socket::{HandOut, Handed, RefuseWhy, connection_role, hand_out};
