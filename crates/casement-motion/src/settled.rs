//! Whether motion has come to rest.

/// Whether a sampled frame is the last one motion needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Settled {
    /// Something still moves: ask for the next frame.
    Moving,
    /// Everything is at rest: stop asking for frames (design/26 R3).
    Rest,
}
