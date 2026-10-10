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

use super::{SpaceBook, SpaceIndex};
use crate::ids::OutputKey;
use porter_core::SpaceId;
use serde::{Deserialize, Serialize};

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
    /// The display it belongs to.
    pub output: OutputKey,
    /// Its place in the display's row.
    pub position: SpaceIndex,
}

impl StoredSpace {
    /// A stored Space.
    pub fn new(id: SpaceId, output: OutputKey, position: SpaceIndex) -> StoredSpace {
        StoredSpace {
            id,
            output,
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
        let _ = text;
        todo!("P1-M2: SpaceFile::parse")
    }

    /// The file as text: one `[[spaces]]` table per Space, in id order, so the same Spaces
    /// always render the same bytes.
    pub fn render(&self) -> Result<String, SpaceFileError> {
        todo!("P1-M2: SpaceFile::render")
    }

    /// The Spaces sill's interim minter wrote to `quire/spaces.json`, read once (C16): its
    /// `by_id` keys become Space ids, with the `output` and `index` a record carries when it
    /// has them. A record without an output is kept without a home and adopted by the first
    /// output plugged.
    pub fn import_sill_json(text: &str) -> Result<SpaceFile, SpaceFileError> {
        let _ = text;
        todo!("P1-M2: SpaceFile::import_sill_json")
    }

    /// What `book` stores: its desktop Spaces and what each output shows.
    pub fn of_book(book: &SpaceBook) -> SpaceFile {
        let _ = book;
        todo!("P1-M2: SpaceFile::of_book")
    }
}
