//! The wait grammar: what `casementctl debug wait` waits for (on session time).
//!
//! ```text
//! mapped APP_ID | focused APP_ID | gone APP_ID | frames N [OUTPUT] | settled
//! ```
//!
//! `frames N --output OUTPUT` (11 section 4.1's spelling) reads as `frames N OUTPUT`.

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
    let words: Vec<&str> = text.split_whitespace().collect();
    let parsed = match words.as_slice() {
        ["mapped", app] => Some(WaitPredicate::Mapped((*app).to_owned())),
        ["focused", app] => Some(WaitPredicate::Focused((*app).to_owned())),
        ["gone", app] => Some(WaitPredicate::Gone((*app).to_owned())),
        ["frames", n] => frames(n, None),
        ["frames", n, "--output", output] | ["frames", n, output] => frames(n, Some(output)),
        ["settled"] => Some(WaitPredicate::Settled),
        _ => None,
    };
    parsed.ok_or_else(|| IpcError::BadLine(text.to_owned()))
}

/// `frames N [OUTPUT]`: at least one frame, on a well-formed output name.
fn frames(n: &str, output: Option<&&str>) -> Option<WaitPredicate> {
    let output = match output {
        Some(name) => Some(OutputName::parse(name).ok()?),
        None => None,
    };
    Some(WaitPredicate::Frames {
        n: n.parse().ok().filter(|n| *n > 0)?,
        output,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
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
            (
                "frames 2 --output HEADLESS-1",
                WaitPredicate::Frames {
                    n: 2,
                    output: Some(OutputName::parse("HEADLESS-1").expect("name")),
                },
            ),
            ("settled", WaitPredicate::Settled),
        ];
        for (text, expected) in rows {
            assert_eq!(parse_wait(text).expect(text), expected, "{text}");
        }
        for text in [
            "",
            "mapped",
            "mapped a b",
            "frames x",
            "frames 0",
            "frames 2 bad name",
            "frames 2 bad.name",
            "settled now",
            "soon",
        ] {
            assert!(
                matches!(parse_wait(text), Err(IpcError::BadLine(_))),
                "{text}"
            );
        }
    }
}
