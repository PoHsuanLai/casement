//! Why an IPC request failed.

/// Why an IPC request failed.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum IpcError {
    /// A line of the inject or wait grammar did not parse.
    #[error("bad line: {0}")]
    BadLine(String),
    /// The bus refused.
    #[error("bus: {0}")]
    Bus(String),
}
