//! A value in a timing line: one word.

use std::fmt;

/// What a line prints for a value that was empty.
const EMPTY: &str = "-";

/// A text that fits a `key=value` field: no whitespace and no control characters (each becomes
/// `_`), and `-` where the text was empty. App ids go through this as the client gave them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token(String);

impl Token {
    /// `text` as one word.
    pub fn of(text: &str) -> Token {
        let word: String = text
            .chars()
            .map(|c| {
                if c.is_whitespace() || c.is_control() {
                    '_'
                } else {
                    c
                }
            })
            .collect();
        Token(if word.is_empty() {
            EMPTY.to_owned()
        } else {
            word
        })
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_value_is_one_word() {
        // name, given, printed
        let cases = [
            ("plain", "org.mozilla.firefox", "org.mozilla.firefox"),
            ("space", "My App", "My_App"),
            ("tab and newline", "a\tb\nc", "a_b_c"),
            ("empty", "", "-"),
            ("kept punctuation", "foot-server=1", "foot-server=1"),
        ];
        for (name, given, printed) in cases {
            assert_eq!(Token::of(given).to_string(), printed, "{name}");
        }
    }
}
