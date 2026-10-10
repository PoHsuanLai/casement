//! The compositor's own shell bindings, read from the desktop's keymap (chordkit's layers:
//! the conventions for our desktop, the machine's keymap, the person's keymap, through
//! keycap-client's `KeycapSource`).
//!
//! - [`chord`]: a chordkit chord as casement's own (pure).
//! - [`shell`]: which standard actions the compositor carries out, and the bindings a keymap
//!   gives them (pure).
//! - [`live`]: the source for a config home, the first load and the reload when the keymap
//!   changes.
//!
//! Chords stay data of the keymap; this module holds action names only (chordkit's
//! `StandardAction`), never raw keys. A `[bindings]` entry in casement's own settings document
//! comes first and so wins over the keymap for the same chord.

mod chord;
mod live;
mod shell;

pub use chord::chord_of;
pub use live::{attach, load_bindings, source_for};
pub use shell::{bindings_of, merge, shell_action};
