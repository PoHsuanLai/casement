//! The wait grammar: what `casementctl debug wait` waits for (on session time).
//!
//! ```text
//! mapped APP_ID | focused APP_ID | gone APP_ID | frames N [OUTPUT] | settled
//! ```

use crate::error::IpcError;
use casement_core::OutputName;

/// What to wait for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WaitPredicate {
    /// A window of this app id is mapped.
    Mapped(String),
    /// A window of this app id has keyboard focus.
    Focused(String),
    /// No window of this app id is left.
    Gone(String),
    /// `n` more frames were presented (on `output`, or on every output).
    Frames {
        /// How many.
        n: u32,
        /// Which output.
        output: Option<OutputName>,
    },
    /// Nothing is moving and no client has a request pending.
    Settled,
}

/// The predicate `text` says.
pub fn parse_wait(text: &str) -> Result<WaitPredicate, IpcError> {
    let _ = text;
    todo!("P0-T-dev: parse_wait")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "P0-T-dev"]
    fn the_wait_table() {
        let output = OutputName::parse("HEADLESS-1").expect("name");
        let rows = [
            ("mapped foot", WaitPredicate::Mapped("foot".to_owned())),
            (
                "focused org.quire.Mail",
                WaitPredicate::Focused("org.quire.Mail".to_owned()),
            ),
            ("gone foot", WaitPredicate::Gone("foot".to_owned())),
            ("frames 3", WaitPredicate::Frames { n: 3, output: None }),
            (
                "frames 2 HEADLESS-1",
                WaitPredicate::Frames {
                    n: 2,
                    output: Some(output),
                },
            ),
            ("settled", WaitPredicate::Settled),
        ];
        for (text, expected) in rows {
            assert_eq!(parse_wait(text).expect(text), expected, "{text}");
        }
        for text in ["", "mapped", "frames x", "frames 2 bad name", "soon"] {
            assert!(
                matches!(parse_wait(text), Err(IpcError::BadLine(_))),
                "{text}"
            );
        }
    }
}
