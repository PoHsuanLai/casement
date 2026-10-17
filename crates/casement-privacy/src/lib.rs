//! casement's privacy files: where the person's book (which program may use which power) and
//! the administrator's floor are kept, how they are read and written safely, and the list of
//! both that `casementctl privacy list` prints. The consent machine itself is pure and lives in
//! `casement_core::privilege`; the composition root (`casement::privacy`) feeds it what this
//! crate reads and writes what it asks to keep.
//!
//! ```text
//! $XDG_STATE_HOME/casement/privacy.toml        the person's book; casement alone writes it
//! /etc/quire/privacy.toml, privacy.d/*.toml    the administrator's floor; casement only reads it
//! ```
//!
//! The book is read when casement starts and never again: the model's copy is the authority and
//! the file is rewritten after every change (atomically, in a folder only the person can enter,
//! mode 0600). A damaged book is kept under another name and an empty one used. A floor file is
//! believed only when it and its folder belong to the administrator alone.

mod floor_files;
mod listing;
mod load;
mod store;
mod system_dir;

pub use floor_files::{DROP_INS_MAX, FloorFiles, Owner};
pub use listing::{Listing, NOBODY_ASKED, list};
pub use load::{load_book, load_floor, save_book};
pub use store::{
    BookSink, Distrust, FilePrivacyStore, FloorOnlyStore, FloorText, NoPrivacyStore, PrivacyFiles,
    PrivacyStore, StoreError,
};
pub use system_dir::SystemDir;

use std::path::{Path, PathBuf};

/// The book's file name, in casement's state folder.
pub const BOOK_FILE: &str = "privacy.toml";

/// The person's book in casement's state folder `dir` (`$XDG_STATE_HOME/casement`).
pub fn book_in(dir: &Path) -> PathBuf {
    dir.join(BOOK_FILE)
}
