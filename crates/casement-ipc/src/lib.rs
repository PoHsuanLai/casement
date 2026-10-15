//! casement's D-Bus servers (C55): `org.quire.Compositor1` (P2-Z) and, under feature
//! `debug-inject`, `org.quire.Casement.Debug1`, which `casementctl debug` drives on the jail's
//! private bus. The inject and wait grammars are pure and always built, so their tables run
//! in every gate. `shell_calls` is the seam bindings call sill through (A4); `space_registry` is
//! the seam the Spaces casement mints are registered with the account service through.

mod error;
mod handles;
mod inject;
mod shot;
mod wait;

pub mod dbus;
pub mod shell_calls;
pub mod signals;
pub mod sleep;
pub mod space_registry;
pub mod system_scheme;

pub use error::IpcError;
pub use handles::IpcHandles;
pub use inject::{DeviceVerb, InjectLine, InjectOrigin, parse_inject};
pub use shot::{ShotArea, parse_shot};
pub use wait::{WaitPredicate, parse_wait};
