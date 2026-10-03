//! The fake palmrest feed: a SEQPACKET socket at `$XDG_RUNTIME_DIR/palmrest/events.sock`
//! replaying recorded `ToHost` lines (palmrest's `replay_to_host` example, P0-PR0). Bodies at
//! P3-Z (palmrest protocol v2).

/// The fake palmrest.
#[derive(Debug, Default)]
pub struct FakePalmrest;
