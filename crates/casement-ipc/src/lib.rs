//! casement's D-Bus servers (C55): `org.quire.Compositor1` (P2-Z) and, under feature
//! `debug-inject`, `org.quire.Casement.Debug1`, which `casementctl debug` drives on the jail's
//! private bus. The inject and wait grammars are pure and always built, so their tables run
//! in every gate.

mod error;
mod handles;
mod inject;
mod wait;

pub mod dbus;

pub use error::IpcError;
pub use handles::IpcHandles;
pub use inject::{DeviceVerb, InjectLine, InjectOrigin, parse_inject};
pub use wait::{WaitPredicate, parse_wait};
