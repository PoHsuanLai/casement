//! A key chord: held modifiers and a keysym. Stored and configured in its text form
//! (`"cmd+space"`); the modifiers are the Mac view of the keyboard, before per-app remapping.

use super::Keysym;
use crate::error::CoreError;
use casement_protocols::vocab::HeldMods;
use serde::{Deserialize, Serialize};

/// Held modifiers and one key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Chord {
    /// The modifiers held.
    pub mods: HeldMods,
    /// The key pressed.
    pub key: Keysym,
}

/// The modifier words, in the Mac's order: how a chord prints them.
const MODIFIER_WORDS: [(&str, HeldMods); 5] = [
    ("ctrl", HeldMods::CONTROL),
    ("option", HeldMods::OPTION),
    ("shift", HeldMods::SHIFT),
    ("cmd", HeldMods::COMMAND),
    ("fn", HeldMods::FUNCTION),
];

impl Chord {
    /// The chord written as `text`: modifier words (`ctrl`, `option`, `shift`, `cmd`, `fn`)
    /// and an XKB keysym name, joined by `+`, in any order (`"cmd+shift+4"`, `"cmd+Tab"`).
    /// The key is the last word; a modifier named twice, an unknown word or a missing key is
    /// an error.
    pub fn parse(text: &str) -> Result<Chord, CoreError> {
        let malformed = || CoreError::Malformed {
            what: "chord",
            text: text.to_owned(),
        };
        let (head, key) = match text.rsplit_once('+') {
            Some((head, key)) => (Some(head), key),
            None => (None, text),
        };
        let key = Keysym::named(key).ok_or_else(malformed)?;
        let mods = head.into_iter().flat_map(|head| head.split('+')).try_fold(
            HeldMods::NONE,
            |held, word| {
                let one = modifier(word).filter(|one| !held.holds(*one))?;
                Some(held.with(one))
            },
        );
        Ok(Chord {
            mods: mods.ok_or_else(malformed)?,
            key,
        })
    }

    /// The chord's text: modifiers in the Mac's order (ctrl, option, shift, cmd, fn), then the
    /// keysym's XKB name. `Chord::parse(&chord.text()) == Ok(chord)` for every chord without
    /// Caps Lock, which is a lock and not part of a chord (it is left out).
    pub fn text(&self) -> String {
        MODIFIER_WORDS
            .iter()
            .filter(|(_, one)| self.mods.holds(*one))
            .map(|(word, _)| (*word).to_owned())
            .chain(std::iter::once(self.key.name()))
            .collect::<Vec<_>>()
            .join("+")
    }
}

fn modifier(word: &str) -> Option<HeldMods> {
    MODIFIER_WORDS
        .iter()
        .find(|(known, _)| known.eq_ignore_ascii_case(word))
        .map(|(_, one)| *one)
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
