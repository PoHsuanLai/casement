//! The Space ids file: a pure codec between the model's Spaces and the text of
//! `$XDG_STATE_HOME/casement/spaces.toml` (A1). Reading and writing that file is the
//! composition root's `SpaceStore` seam; nothing here touches a file (B1).
//!
//! What survives a restart is the Space ids (they are ext-workspace's `id`, so a client that
//! remembers one finds it again), each Space's output and place in the row, and which Space
//! each output showed. Windows do not survive, so only desktops are stored.
//!
//! ```no_run
//! use casement_core::space::SpaceFile;
//!
//! let file = SpaceFile::parse("").expect("an empty file is no Spaces");
//! assert_eq!(SpaceFile::parse(&file.render().expect("render")).expect("parse"), file);
//! ```

use super::{SpaceBook, SpaceContent, SpaceIndex};
use crate::ids::OutputKey;
use porter_core::SpaceId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The Spaces file.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
#[non_exhaustive]
pub struct SpaceFile {
    /// Every stored desktop Space.
    pub spaces: Vec<StoredSpace>,
    /// The Space each output showed.
    pub shown: Vec<StoredShown>,
}

/// One stored Space.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct StoredSpace {
    /// Its id.
    pub id: SpaceId,
    /// The display it belongs to; none for a Space imported without a home, which the first
    /// output plugged adopts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<OutputKey>,
    /// Its place in the display's row.
    pub position: SpaceIndex,
}

impl StoredSpace {
    /// A stored Space of `output`.
    pub fn new(id: SpaceId, output: OutputKey, position: SpaceIndex) -> StoredSpace {
        StoredSpace {
            id,
            output: Some(output),
            position,
        }
    }

    /// A stored Space with no home output yet.
    pub fn homeless(id: SpaceId, position: SpaceIndex) -> StoredSpace {
        StoredSpace {
            id,
            output: None,
            position,
        }
    }
}

/// The Space an output showed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct StoredShown {
    /// The display.
    pub output: OutputKey,
    /// The Space it showed.
    pub space: SpaceId,
}

impl StoredShown {
    /// A stored choice.
    pub fn new(output: OutputKey, space: SpaceId) -> StoredShown {
        StoredShown { output, space }
    }
}

/// Why the file's text could not be read or written.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SpaceFileError {
    /// `spaces.toml` did not parse.
    #[error("spaces.toml: {0}")]
    Parse(#[from] toml::de::Error),
    /// The file could not be rendered.
    #[error("spaces.toml: {0}")]
    Render(#[from] toml::ser::Error),
    /// An interim `spaces.json` from sill did not parse.
    #[error("spaces.json: {0}")]
    SillJson(#[from] serde_json::Error),
}

impl SpaceFile {
    /// A file with no Spaces.
    pub fn empty() -> SpaceFile {
        SpaceFile::default()
    }

    /// A file of these Spaces and choices.
    pub fn new(spaces: Vec<StoredSpace>, shown: Vec<StoredShown>) -> SpaceFile {
        SpaceFile { spaces, shown }
    }

    /// The file `text` holds (empty text is an empty file; keys it does not know are ignored).
    pub fn parse(text: &str) -> Result<SpaceFile, SpaceFileError> {
        Ok(toml::from_str(text)?)
    }

    /// The file as text: one `[[spaces]]` table per Space, in id order, so the same Spaces
    /// always render the same bytes.
    pub fn render(&self) -> Result<String, SpaceFileError> {
        let mut sorted = self.clone();
        sorted.spaces.sort_by(|a, b| a.id.cmp(&b.id));
        sorted.shown.sort_by(|a, b| a.output.cmp(&b.output));
        Ok(toml::to_string(&sorted)?)
    }

    /// The Spaces sill's interim minter wrote to `quire/spaces.json`, read once (C16): its
    /// `by_id` keys become Space ids, with the `output` and `index` a record carries when it
    /// has them. A record without an output is kept without a home and adopted by the first
    /// output plugged.
    ///
    /// sill writes no such file yet (its `look.rs` says quire has no store), so the shape is
    /// the minimal one the codec pins: `{"by_id": {"<space id>": {"output": "<output key>",
    /// "index": 0}}}`, both record fields optional, unknown keys ignored.
    pub fn import_sill_json(text: &str) -> Result<SpaceFile, SpaceFileError> {
        let parsed: SillFile = serde_json::from_str(text)?;
        let mut spaces = Vec::with_capacity(parsed.by_id.len());
        for (at, (key, record)) in parsed.by_id.into_iter().enumerate() {
            let id = SpaceId::parse(&key).map_err(|error| {
                <serde_json::Error as serde::de::Error>::custom(format!(
                    "space id {key:?}: {error:?}"
                ))
            })?;
            let position = SpaceIndex(
                record
                    .index
                    .unwrap_or_else(|| u32::try_from(at).unwrap_or(u32::MAX)),
            );
            spaces.push(match record.output {
                Some(output) => StoredSpace::new(id, output, position),
                None => StoredSpace::homeless(id, position),
            });
        }
        Ok(SpaceFile::new(spaces, Vec::new()))
    }

    /// What `book` stores: its desktop Spaces and what each output shows.
    pub fn of_book(book: &SpaceBook) -> SpaceFile {
        let is_desktop = |id: &SpaceId| {
            book.get(id)
                .is_some_and(|space| matches!(space.content, SpaceContent::Desktop))
        };
        let spaces = book
            .iter()
            .filter(|space| is_desktop(&space.id))
            .map(|space| StoredSpace::new(space.id.clone(), space.output.clone(), space.position))
            .collect();
        let shown = book
            .shown()
            .filter(|(_, id)| is_desktop(id))
            .map(|(output, id)| StoredShown::new(output.clone(), id.clone()))
            .collect();
        SpaceFile::new(spaces, shown)
    }
}

/// The interim `spaces.json` of sill (read once).
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct SillFile {
    by_id: BTreeMap<String, SillRecord>,
}

/// One record of it.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct SillRecord {
    output: Option<OutputKey>,
    index: Option<u32>,
}
