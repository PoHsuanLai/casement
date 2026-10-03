//! Which input serials answer a real press: the proof a move, resize, popup grab or activation
//! request must carry (03 §1.2, A7).

use casement_core::input::Origin;
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
        let _ = (serial, press);
        todo!("P0-H4: SerialBook::record")
    }

    /// The press `serial` answers, if it is recent and was delivered to `client`.
    pub fn answers(&self, serial: InputSerial, client: ClientKey) -> Option<&PressRecord> {
        let _ = (serial, client);
        todo!("P0-H4: SerialBook::answers")
    }
}
