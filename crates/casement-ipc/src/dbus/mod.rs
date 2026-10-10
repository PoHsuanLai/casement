//! The D-Bus objects. `compositor1` (`org.quire.Compositor1`) joins at P2-Z.

#[cfg(feature = "debug-inject")]
mod debug1;

#[cfg(feature = "debug-inject")]
mod serve;

#[cfg(feature = "debug-inject")]
pub use serve::serve_debug_at;

#[cfg(feature = "debug-inject")]
pub use debug1::{Debug1, DebugCall, DebugReply, DebugRequest, OBJECT_PATH};
