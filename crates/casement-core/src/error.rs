//! The crate's one error: text from outside that does not parse.

/// Why a value from outside (a settings string, a stored file, a wire string) was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum CoreError {
    /// `text` is not a well-formed `what`.
    #[error("malformed {what}: {text}")]
    Malformed {
        /// What the text was meant to be (`"output name"`, `"chord"`).
        what: &'static str,
        /// The text as it arrived.
        text: String,
    },
}
