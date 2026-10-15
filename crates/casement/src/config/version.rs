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
use crate::cli::ConfigHome;
use casement_core::settings::Enabled;
use serde::{Deserialize, Serialize};

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

/// What stamping the version did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Stamped {
    /// There is no file, or it already says its version: nothing written.
    Left,
    /// The file was migrated when read; it now begins with the current version, so the next
    /// read does not drop a value the person sets to a retired default.
    Written,
}

/// Writes the current version at the top of `home`'s settings file when the file has none, and
/// nothing else of it changes (comments, unknown keys and the order stay as the person wrote
/// them). Without it a person who sets a retired default on purpose (natural scrolling off for
/// a mouse) would lose the choice at the next start.
///
/// ```no_run
/// use casement::cli::ConfigHome;
/// use casement::config::stamp_version;
///
/// let _ = stamp_version(&ConfigHome::Xdg);
/// ```
pub fn stamp_version(home: &ConfigHome) -> Result<Stamped, ConfigError> {
    let store = store(home);
    let Some(path) = store.path::<CasementDoc>() else {
        return Ok(Stamped::Left);
    };
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Ok(Stamped::Left);
    };
    if store.load::<CasementDoc>().value.version.is_some() {
        return Ok(Stamped::Left);
    }
    let stamped = format!("version = {}\n{text}", Version::CURRENT.0);
    let part = path.with_extension("part");
    let unwritten = |error: std::io::Error| ConfigError::Settings(error.to_string());
    std::fs::write(&part, stamped).map_err(unwritten)?;
    std::fs::rename(&part, &path).map_err(unwritten)?;
    Ok(Stamped::Written)
}
