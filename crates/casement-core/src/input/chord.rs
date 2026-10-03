//! A key chord: held modifiers and a keysym. Stored and configured in its text form
//! (`"cmd+space"`); the modifiers are the Mac view of the keyboard, before per-app remapping.

use super::Keysym;
use crate::error::CoreError;
use casement_protocols::vocab::MacMods;
use serde::{Deserialize, Serialize};

/// Held modifiers and one key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Chord {
    /// The modifiers held.
    pub mods: MacMods,
    /// The key pressed.
    pub key: Keysym,
}

impl Chord {
    /// The chord written as `text`: modifier words (`ctrl`, `option`, `shift`, `cmd`, `fn`)
    /// and an XKB keysym name, joined by `+`, in any order (`"cmd+shift+4"`, `"cmd+Tab"`).
    pub fn parse(text: &str) -> Result<Chord, CoreError> {
        let _ = text;
        todo!("P0-F1-input: modifier words and a keysym name")
    }

    /// The chord's text: modifiers in the Mac's order (ctrl, option, shift, cmd, fn), then the
    /// keysym's XKB name. `Chord::parse(&chord.text()) == Ok(chord)`.
    pub fn text(&self) -> String {
        todo!("P0-F1-input: the inverse of parse")
    }
}

impl TryFrom<String> for Chord {
    type Error = CoreError;
    fn try_from(text: String) -> Result<Chord, CoreError> {
        Chord::parse(&text)
    }
}

impl From<Chord> for String {
    fn from(chord: Chord) -> String {
        chord.text()
    }
}
