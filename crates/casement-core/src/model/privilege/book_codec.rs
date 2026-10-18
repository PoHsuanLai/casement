//! The book as text: `$XDG_STATE_HOME/casement/privacy.toml`. A pure codec: reading and
//! writing the file is the composition root's seam. Rows that do not parse are dropped (an
//! answer lost is a question asked again, never a program let in); a file that is not this
//! version, too large or not text of this shape is an error, and the root sets it aside and
//! starts from an empty book.
//!
//! ```toml
//! version = 1
//! [[grant]]
//! power  = "screen"
//! answer = "allow"
//! at     = 1760619000
//! subject = { program = "/usr/bin/grim" }
//! ```

use super::book::{Book, Grant, UnixSeconds, Verdict};
use super::limits::{MAX_FILE_BYTES, MAX_GRANTS};
use super::subject_dto::SubjectDto;
use casement_protocols::vocab::Power;
use serde::{Deserialize, Serialize};

/// The only version this build reads and writes.
const VERSION: i64 = 1;

/// A file read from text (a book, the Spaces file), and how many rows were dropped to make it
/// one casement would have written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Loaded<T> {
    /// What was read.
    pub value: T,
    /// The rows dropped: unreadable, or past the cap.
    pub dropped: usize,
}

/// Why the book's text could not be read or written.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum BookError {
    /// The text is larger than a book can be.
    #[error("privacy.toml is larger than 64 KiB")]
    TooLarge,
    /// The text is not this shape.
    #[error("privacy.toml: {0}")]
    Parse(#[from] toml::de::Error),
    /// The book could not be written as text.
    #[error("privacy.toml: {0}")]
    Render(#[from] toml::ser::Error),
    /// The file is from another version.
    #[error("privacy.toml has version {0}, this build reads version 1")]
    Version(i64),
}

#[derive(Deserialize)]
struct InFile {
    version: i64,
    #[serde(default)]
    grant: Vec<toml::Value>,
}

#[derive(Serialize)]
struct OutFile {
    version: i64,
    grant: Vec<GrantDto>,
}

/// One row. The subject is last: a table follows the plain values of its row.
#[derive(Serialize, Deserialize)]
struct GrantDto {
    power: Power,
    answer: Verdict,
    at: i64,
    subject: SubjectDto,
}

impl GrantDto {
    fn of(grant: &Grant) -> GrantDto {
        GrantDto {
            power: grant.power,
            answer: grant.verdict,
            at: i64::try_from(grant.at.0).unwrap_or(i64::MAX),
            subject: SubjectDto::of(&grant.subject),
        }
    }

    fn grant(self) -> Option<Grant> {
        Some(Grant {
            power: self.power,
            subject: self.subject.subject()?,
            verdict: self.answer,
            at: UnixSeconds(u64::try_from(self.at).unwrap_or(0)),
        })
    }
}

impl Book {
    /// The book `text` holds: an empty text is an error (the file always has its version),
    /// keys it does not know are dropped, at most [`MAX_GRANTS`] rows are read, and a later
    /// row for the same program and power replaces an earlier one.
    pub fn from_toml(text: &str) -> Result<Loaded<Book>, BookError> {
        if text.len() > MAX_FILE_BYTES {
            return Err(BookError::TooLarge);
        }
        let file: InFile = toml::from_str(text)?;
        if file.version != VERSION {
            return Err(BookError::Version(file.version));
        }
        let rows = file.grant.len();
        let grants: Vec<Grant> = file
            .grant
            .into_iter()
            .take(MAX_GRANTS)
            .filter_map(|row| row.try_into::<GrantDto>().ok())
            .filter_map(GrantDto::grant)
            .collect();
        let dropped = rows - grants.len();
        let book = grants.into_iter().fold(Book::empty(), Book::with);
        Ok(Loaded {
            value: book,
            dropped,
        })
    }

    /// The book as text: one `[[grant]]` per answer, in power then program order, so the same
    /// answers always render the same bytes.
    pub fn to_toml(&self) -> Result<String, BookError> {
        let file = OutFile {
            version: VERSION,
            grant: self.grants().iter().map(GrantDto::of).collect(),
        };
        Ok(toml::to_string(&file)?)
    }
}
