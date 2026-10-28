//! The D-Bus objects: `compositor1` (`org.quire.Compositor1`, P2-Z; its callers are proven in
//! `callers` and counted in `intake`) and, under feature `debug-inject`, `debug1`.

mod callers;
mod compositor1;
mod compositor_serve;
mod connect_ask;
mod intake;
mod portal_watch;

pub use callers::{CallerCheck, DesktopCallers, WhoFuture, require, role_for_connection};
pub use compositor_serve::{CallerSetup, serve_compositor_at};
pub use compositor1::{
    BUS_NAME as COMPOSITOR_BUS_NAME, Compositor1, OBJECT_PATH as COMPOSITOR_OBJECT_PATH,
    parse_connect_role,
};
pub use connect_ask::{
    ConnectAsk, ConnectCall, ConnectRefusal, ConnectReply, EndAsk, EndLeaseAsk, FocusLeaseAsk,
    GoneAsk, LeaseAsk, NOT_THE_CALLER, Proven, RemoteAsk,
};
pub use intake::{
    ANSWER_WAIT, CALLS_PER_WINDOW, CONNECT_QUEUE, Intake, RATE_WINDOW, RateLimit, TRACKED_CALLERS,
};
/// The caller table and the files it is read from (the caller-identity library's), so the
/// composition root names no second crate for them.
pub use peephole_bus::peephole::{LoadError, RealProc, TableFiles};
pub use peephole_bus::peephole_core::Table as CallerTable;

/// What the composition root needs to say who the program on a Wayland socket is (the same
/// library's, for the same reason): hold the peer, read its facts, judge them against the table.
pub mod peer {
    pub use peephole_bus::peephole::{Alive, PinError, Pinned, ProbeError, Prober};
    pub use peephole_bus::peephole_core::{
        AppKey, Engine, Exe, ExePath, Facts, FileTrust, Pin, Proof, RoleName, Standing, Table, Uid,
        WhyNot, judge, roles,
    };
}

#[cfg(feature = "debug-inject")]
mod debug1;

#[cfg(feature = "debug-inject")]
mod debug_units;

#[cfg(feature = "debug-inject")]
pub use debug_units::{
    DebugPeerUnits, PeerMatch, PeerUnitRule, PeerUnitTable, PeerUnitsError, ProcComm,
};

#[cfg(feature = "debug-inject")]
mod serve;

#[cfg(feature = "debug-inject")]
pub use serve::serve_debug_at;

#[cfg(feature = "debug-inject")]
pub use debug1::{Debug1, DebugCall, DebugReply, DebugRequest, OBJECT_PATH};
