//! Pumping a client.

/// Whether a pump moved anything (the fixture pumps until every party is quiet).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Pumped {
    /// Something was read, dispatched or sent.
    Moved,
    /// Nothing happened.
    Quiet,
}
