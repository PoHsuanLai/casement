//! The settings file's version, and the migration from the files written before it had one.
//!
//! A default that changes would not reach a person whose file holds the old default (a file
//! written by a save holds every key). Version 2 is the first to say it: a file with no version,
//! or an older one, has each key that still holds the value that was the default then dropped
//! (back to the default now), and a key the person set to anything else is kept. Saving a
//! migrated document writes the current version, so this happens once.

use super::ConfigError;
use super::doc::{CasementDoc, store};
use super::sections::PointerSection;
use casement_core::settings::Enabled;
use ds_helpers::{NewFileMode, ReadCap, read_text_capped, write_atomic};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// The largest settings file read here: the cap the settings library reads it with, and a FIFO or
/// a device at the path is refused rather than waited on, so start-up cannot hang on it (CASB-13).
const SETTINGS_FILE_CAP: ReadCap = ReadCap::mib(1);

/// The version of the settings file; a file without the key is older than any version.
///
/// ```
/// use casement::config::Version;
///
/// assert!(Version(1) < Version::CURRENT);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Version(pub u32);

impl Version {
    /// The version casement writes: 2 made natural scrolling on for mice (casement 8b10803).
    pub const CURRENT: Version = Version(2);
}

/// What `pointer.natural_scroll` defaulted to before version 2.
const OLD_MOUSE_NATURAL_SCROLL: Enabled = Enabled::Off;

/// `held`, unless it is the retired default `old`: then the default now, `fresh`.
fn unless_retired<T: PartialEq>(held: T, old: T, fresh: T) -> T {
    if held == old { fresh } else { held }
}

impl CasementDoc {
    /// This document brought to the current version: from an older one, the keys that still hold
    /// a retired default take the new default and the rest stay as the person set them.
    ///
    /// ```
    /// use casement::config::{CasementDoc, Version};
    ///
    /// let doc = CasementDoc::default();
    /// assert_eq!(doc.version, None);
    /// assert_eq!(doc.migrated().version, Some(Version::CURRENT));
    /// ```
    pub fn migrated(self) -> CasementDoc {
        if self
            .version
            .is_some_and(|version| version >= Version::CURRENT)
        {
            return self;
        }
        let fresh = PointerSection::default();
        CasementDoc {
            version: Some(Version::CURRENT),
            pointer: PointerSection {
                natural_scroll: unless_retired(
                    self.pointer.natural_scroll,
                    OLD_MOUSE_NATURAL_SCROLL,
                    fresh.natural_scroll,
                ),
                ..self.pointer
            },
            ..self
        }
    }
}

/// Writes the current version at the top of `home`'s settings file when the file has none, and
/// nothing else of it changes (comments, unknown keys and the order stay as the person wrote
/// them). Whether it wrote: `false` when there is no file, or it already says its version; `true`
/// when the file was migrated when read and now begins with the current version, so the next
/// read does not drop a value the person sets to a retired default. Without it a person who sets a retired default on purpose (natural scrolling off for
/// a mouse) would lose the choice at the next start.
///
/// ```no_run
/// use casement::config::stamp_version;
///
/// let _ = stamp_version(None);
/// ```
pub fn stamp_version(home: Option<&Path>) -> Result<bool, ConfigError> {
    let store = store(home);
    let Some(path) = store.path::<CasementDoc>() else {
        return Ok(false);
    };
    let Ok(text) = read_text_capped(&path, SETTINGS_FILE_CAP) else {
        return Ok(false);
    };
    if store.load::<CasementDoc>().value.version.is_some() {
        return Ok(false);
    }
    let stamped = format!("version = {}\n{text}", Version::CURRENT.0);
    // Quire's writer: a unique temporary file beside it, synced and renamed over it, the file's
    // own permissions kept and a link at the path followed (a dotfile-managed file stays a link).
    write_atomic(&path, stamped.as_bytes(), NewFileMode::OwnerOnly)
        .map_err(|error| ConfigError::Settings(error.to_string()))?;
    Ok(true)
}
