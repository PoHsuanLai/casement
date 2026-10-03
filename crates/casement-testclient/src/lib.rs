//! casement's test clients: typed wayland-client helpers per protocol, the [`Transcript`] every
//! client keeps (each event with its virtual instant), test [`Pattern`]s and shm buffers.
//! Reaches no smithay (row B9): the clients speak only the wire.
//!
//! The module tree is frozen; each protocol's helper module is filled by the lane that owns
//! the protocol (20-FREEZE-PLAN §5.2), the infrastructure (`conn`, `globals`, `pump`,
//! `transcript`, `pattern`, `shm`, `udmabuf`) by P0-T-client.

mod conn;
mod globals;
mod pattern;
mod pump;
mod shm;
mod transcript;
pub mod udmabuf;

pub mod capture;
pub mod ime;
pub mod manage;
pub mod misc;
pub mod quire;
pub mod seat;
pub mod selection;
pub mod surface;

pub use conn::{BaseData, ClientError, ClientProtocolError, ClientState, SyncDone, TestClient};
pub use globals::{Global, GlobalList, GlobalListContents};
pub use pattern::Pattern;
pub use pump::Pumped;
pub use shm::{ShmBuffer, shm_buffer};
pub use transcript::{Transcript, TranscriptLine};
