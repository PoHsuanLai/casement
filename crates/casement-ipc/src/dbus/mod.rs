//! The D-Bus objects: `compositor1` (`org.quire.Compositor1`, P2-Z) and, under feature
//! `debug-inject`, `debug1`.

mod compositor1;
mod compositor_serve;

pub use compositor_serve::serve_compositor_at;
pub use compositor1::{
    BUS_NAME as COMPOSITOR_BUS_NAME, Compositor1, ConnectAsk, ConnectCall, ConnectRefusal,
    ConnectReply, EndAsk, OBJECT_PATH as COMPOSITOR_OBJECT_PATH, PeerCgroups, ProcCgroups,
    RemoteAsk, parse_connect_role,
};

#[cfg(feature = "debug-inject")]
mod debug1;

#[cfg(feature = "debug-inject")]
mod debug_units;

#[cfg(feature = "debug-inject")]
pub use debug_units::{
    DebugPeerUnits, PeerMatch, PeerNames, PeerUnitRule, PeerUnitTable, PeerUnitsError, ProcComm,
};

#[cfg(feature = "debug-inject")]
mod serve;

#[cfg(feature = "debug-inject")]
pub use serve::serve_debug_at;

#[cfg(feature = "debug-inject")]
pub use debug1::{Debug1, DebugCall, DebugReply, DebugRequest, OBJECT_PATH};
