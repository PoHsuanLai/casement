//! Which input serials answer a real press: the proof a move, resize, popup grab or activation
//! request must carry (03 §1.2, A7).

use casement_core::input::{ButtonCode, Origin};
use casement_core::time::Mono;
use casement_core::{ClientKey, InputSerial};
use std::collections::BTreeMap;

/// A press the seat delivered, as its serial remembers it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PressRecord {
    /// The client the press was delivered to.
    pub client: ClientKey,
    /// When.
    pub at: Mono,
    /// What kind of press.
    pub kind: PressKind,
    /// Where it came from.
    pub origin: Origin,
    /// The pointer button, for a button press: the one a grab it starts ends with.
    pub button: Option<ButtonCode>,
}

/// What kind of press a serial answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PressKind {
    /// A pointer button.
    Button,
    /// A key.
    Key,
    /// A touch down.
    Touch,
    /// A tablet tool tip.
    TabletTip,
}

/// The recent presses by serial (bounded: the oldest are forgotten).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SerialBook {
    presses: BTreeMap<InputSerial, PressRecord>,
}

impl SerialBook {
    /// No presses.
    pub fn empty() -> SerialBook {
        SerialBook::default()
    }

    /// The book with `serial` recorded as `press` (forgetting the oldest beyond the bound).
    pub fn record(self, serial: InputSerial, press: PressRecord) -> SerialBook {
        let mut presses = self.presses;
        presses.insert(serial, press);
        while presses.len() > REMEMBERED {
            if presses.pop_first().is_none() {
                break;
            }
        }
        SerialBook { presses }
    }

    /// The pointer button the press `serial` answers was made with, if it was a button press.
    pub fn button_of(&self, serial: InputSerial) -> Option<ButtonCode> {
        self.presses.get(&serial).and_then(|press| press.button)
    }

    /// The press `serial` answers, if it is recent and was delivered to `client`.
    pub fn answers(&self, serial: InputSerial, client: ClientKey) -> Option<&PressRecord> {
        self.presses
            .get(&serial)
            .filter(|press| press.client == client)
    }
}

/// How many presses the book keeps: a client answers its latest press, and a few more are
/// forgiven for clients that lag a frame or two behind a fast typist.
const REMEMBERED: usize = 64;

#[cfg(test)]
mod tests {
    use super::*;
    use casement_core::input::Origin;

    fn press(client: u64) -> PressRecord {
        PressRecord {
            client: ClientKey(client),
            at: Mono(std::time::Duration::from_millis(5)),
            kind: PressKind::Button,
            origin: Origin::Test,
            button: None,
        }
    }

    #[test]
    fn a_serial_answers_only_the_client_it_was_delivered_to() {
        let book = SerialBook::empty().record(InputSerial(10), press(1));
        assert_eq!(book.answers(InputSerial(10), ClientKey(1)), Some(&press(1)));
        assert_eq!(book.answers(InputSerial(10), ClientKey(2)), None);
        assert_eq!(book.answers(InputSerial(11), ClientKey(1)), None);
    }

    #[test]
    fn the_oldest_serials_are_forgotten_beyond_the_bound() {
        let book = (1..=70u32).fold(SerialBook::empty(), |book, n| {
            book.record(InputSerial(n), press(1))
        });
        assert_eq!(book.answers(InputSerial(6), ClientKey(1)), None);
        assert!(book.answers(InputSerial(7), ClientKey(1)).is_some());
        assert!(book.answers(InputSerial(70), ClientKey(1)).is_some());
    }

    #[test]
    fn recording_the_same_serial_again_replaces_it() {
        let book = SerialBook::empty()
            .record(InputSerial(3), press(1))
            .record(InputSerial(3), press(2));
        assert_eq!(book.answers(InputSerial(3), ClientKey(1)), None);
        assert!(book.answers(InputSerial(3), ClientKey(2)).is_some());
    }
}
