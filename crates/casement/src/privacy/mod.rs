//! Per-app consent, the composition root's half: how the compositor and Settings keep in step
//! with the model's book (`casement_core::privilege`), and where the two privacy files are
//! (`casement-privacy`: the person's book, the administrator's floor).
//!
//! The book is read when casement starts and never again: the model's copy is the authority
//! and the file is rewritten after every change. The floor is read at start and again when
//! casement is told to (`casementctl privacy reload`, which sends `SIGHUP`).

mod host;

pub use casement_privacy::{
    FilePrivacyStore, FloorFiles, FloorText, Listing, NoPrivacyStore, Owner, PrivacyFiles,
    PrivacyStore, StoreError, SystemDir, list, load_book, load_floor, save_book,
};
pub use host::{edited, keep, publish, reload_floor, start};

/// The floor in `system`: `/etc/quire`, or an injected folder whose files this user owns in a
/// build for the jail (a test or a scenario cannot make a root-owned floor) and the
/// administrator anywhere else.
pub fn floor_in(system: &SystemDir) -> FloorFiles {
    system.floor(injected_owner())
}

/// Who owns a floor in an injected folder.
fn injected_owner() -> Owner {
    #[cfg(any(feature = "testing", feature = "debug-inject"))]
    {
        Owner::User(rustix::process::getuid().as_raw())
    }
    #[cfg(not(any(feature = "testing", feature = "debug-inject")))]
    {
        Owner::Root
    }
}
