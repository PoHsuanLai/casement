//! The seam the person's book and the administrator's floor are stored through: casement reads
//! the floor, reads and writes the book, and nothing else touches either file. The real store
//! is [`FilePrivacyStore`]; a run that keeps nothing between runs has [`NoPrivacyStore`].

use super::floor_files::FloorFiles;
use casement_core::privilege::UnixSeconds;
use ds_helpers::{
    NewFileMode, ReadCap, ReadError, WriteError, move_aside, read_capped, write_atomic,
};
use std::fmt::Debug;
use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};

/// The largest book or floor file read (the model's own limit, `MAX_FILE_BYTES`).
pub(super) const FILE_CAP: ReadCap = ReadCap::kib(64);

/// Why a privacy file could not be read or written.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum StoreError {
    /// The file could not be read as a small text file.
    #[error("{0}")]
    Read(#[from] ReadError),
    /// The file could not be written.
    #[error("{0}")]
    Write(#[from] WriteError),
    /// A file or folder could not be looked at or made private.
    #[error("{path}: {source}")]
    Io {
        /// The file or folder.
        path: PathBuf,
        /// Why.
        source: std::io::Error,
    },
    /// A floor file that is not the administrator's alone is not believed.
    #[error("{path}: {why}")]
    Untrusted {
        /// The file or the folder it is in.
        path: PathBuf,
        /// What is wrong with it.
        why: Distrust,
    },
}

/// Why an administrator's file was not believed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Distrust {
    /// Someone other than the administrator owns it.
    NotTheirs,
    /// Someone other than the administrator could change it.
    Writable,
    /// It is not a plain file (or the folder is not a folder).
    NotPlain,
}

impl std::fmt::Display for Distrust {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Distrust::NotTheirs => "is not owned by the administrator alone",
            Distrust::Writable => "can be changed by someone other than the administrator",
            Distrust::NotPlain => "is not a plain file",
        })
    }
}

/// One floor file as read: where it is, and its text or why it is not believed.
#[derive(Debug)]
pub struct FloorText {
    /// The file.
    pub path: PathBuf,
    /// Its text, or why it was refused.
    pub text: Result<String, StoreError>,
}

/// Where the book and the floor are kept.
pub trait PrivacyStore: Debug {
    /// The book's text, or none when there is no book yet. A book that cannot be read is an
    /// error; the caller sets it aside.
    fn read_book(&mut self) -> Result<Option<String>, StoreError>;

    /// Keeps the damaged book under another name, so nothing is lost and a new book can start.
    fn set_book_aside(&mut self, at: UnixSeconds) -> Result<(), StoreError>;

    /// Replaces the book with `text`, atomically.
    fn write_book(&mut self, text: &str) -> Result<(), StoreError>;

    /// Every floor file there is, the main one first and then the drop-ins by name. A file
    /// that is missing is not listed.
    fn read_floor(&mut self) -> Vec<FloorText>;

    /// Whether the program installed at `path` is still there (a full book forgets the
    /// answers about programs that are gone).
    fn program_exists(&self, path: &str) -> bool {
        let _ = path;
        true
    }
}

/// A store that keeps nothing: no book is found, none is written, there is no floor. The
/// answers last the run.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoPrivacyStore;

impl PrivacyStore for NoPrivacyStore {
    fn read_book(&mut self) -> Result<Option<String>, StoreError> {
        Ok(None)
    }

    fn set_book_aside(&mut self, at: UnixSeconds) -> Result<(), StoreError> {
        let _ = at;
        Ok(())
    }

    fn write_book(&mut self, text: &str) -> Result<(), StoreError> {
        let _ = text;
        Ok(())
    }

    fn read_floor(&mut self) -> Vec<FloorText> {
        Vec::new()
    }
}

/// Where the two files live.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivacyFiles {
    /// The person's book (`$XDG_STATE_HOME/casement/privacy.toml`).
    pub book: PathBuf,
    /// The administrator's floor.
    pub floor: FloorFiles,
}

/// The book and the floor as files.
#[derive(Debug)]
pub struct FilePrivacyStore {
    files: PrivacyFiles,
}

impl FilePrivacyStore {
    /// The store over `files`.
    pub fn new(files: PrivacyFiles) -> FilePrivacyStore {
        FilePrivacyStore { files }
    }

    /// The folder the book is in, private to the person (0700). A folder open to others is
    /// closed; one that cannot be made private is an error, because the book lists what their
    /// programs may do.
    fn private_folder(&self) -> Result<(), StoreError> {
        let Some(dir) = self
            .files
            .book
            .parent()
            .filter(|dir| !dir.as_os_str().is_empty())
        else {
            return Ok(());
        };
        let folder = |source: std::io::Error| StoreError::Io {
            path: dir.to_path_buf(),
            source,
        };
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(dir)
            .map_err(folder)?;
        let meta = std::fs::metadata(dir).map_err(folder)?;
        if meta.mode() & 0o077 != 0 {
            std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))
                .map_err(folder)?;
        }
        Ok(())
    }
}

impl PrivacyStore for FilePrivacyStore {
    fn read_book(&mut self) -> Result<Option<String>, StoreError> {
        match read_capped(&self.files.book, FILE_CAP) {
            Ok(bytes) => String::from_utf8(bytes)
                .map(Some)
                .map_err(|source| ReadError::NotText {
                    path: self.files.book.clone(),
                    source,
                })
                .map_err(StoreError::from),
            Err(error) if error.is_missing() => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    fn set_book_aside(&mut self, at: UnixSeconds) -> Result<(), StoreError> {
        let kept = move_aside(&self.files.book, ds_helpers::UnixSeconds(at.0))?;
        tracing::warn!(
            kept = %kept.display(),
            "the privacy book was damaged; it was kept under another name and a new one started"
        );
        Ok(())
    }

    fn write_book(&mut self, text: &str) -> Result<(), StoreError> {
        self.private_folder()?;
        write_atomic(&self.files.book, text.as_bytes(), NewFileMode::OwnerOnly)?;
        Ok(())
    }

    fn read_floor(&mut self) -> Vec<FloorText> {
        self.files.floor.read()
    }

    fn program_exists(&self, path: &str) -> bool {
        Path::new(path).exists()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::load::{load_book, save_book};
    use casement_core::privilege::{Book, Grant, ProgramPath, Subject, Verdict};
    use casement_protocols::vocab::Power;

    fn store_in(dir: &Path) -> FilePrivacyStore {
        FilePrivacyStore::new(PrivacyFiles {
            book: dir.join("state/casement/privacy.toml"),
            floor: FloorFiles::system(),
        })
    }

    fn book() -> Book {
        Book::empty().with(Grant {
            power: Power::Screen,
            subject: Subject::Program(ProgramPath::parse("/usr/bin/grim").expect("a path")),
            verdict: Verdict::Allow,
            at: UnixSeconds(1_760_000_000),
        })
    }

    #[test]
    fn the_book_is_kept_in_a_private_folder_and_read_back() {
        let scratch = tempfile::tempdir().expect("a scratch folder");
        let mut store = store_in(scratch.path());
        assert!(matches!(store.read_book(), Ok(None)), "no book yet");
        save_book(&mut store, &book());
        let folder = scratch.path().join("state/casement");
        let file = folder.join("privacy.toml");
        let mode = |path: &Path| std::fs::metadata(path).expect("there").mode() & 0o777;
        assert_eq!(mode(&folder), 0o700, "the folder");
        assert_eq!(mode(&file), 0o600, "the file");
        assert_eq!(load_book(&mut store, UnixSeconds(5)), book());
    }

    #[test]
    fn a_folder_open_to_others_is_closed_when_the_book_is_written() {
        let scratch = tempfile::tempdir().expect("a scratch folder");
        let folder = scratch.path().join("state/casement");
        std::fs::create_dir_all(&folder).expect("a folder");
        std::fs::set_permissions(&folder, std::fs::Permissions::from_mode(0o755)).expect("open");
        save_book(&mut store_in(scratch.path()), &book());
        assert_eq!(
            std::fs::metadata(&folder).expect("there").mode() & 0o777,
            0o700
        );
    }

    #[test]
    fn a_damaged_or_oversized_book_is_kept_aside_under_the_date_and_an_empty_one_used() {
        let scratch = tempfile::tempdir().expect("a scratch folder");
        let mut store = store_in(scratch.path());
        let folder = scratch.path().join("state/casement");
        std::fs::create_dir_all(&folder).expect("a folder");
        let file = folder.join("privacy.toml");
        let damaged: [(&str, Vec<u8>); 3] = [
            ("not text", vec![0xff, 0xfe, 0x00]),
            ("not this shape", b"version = 7\n".to_vec()),
            ("too big", vec![b'#'; 70 * 1024]),
        ];
        for (case, bytes) in damaged {
            std::fs::write(&file, &bytes).expect("a book");
            assert_eq!(
                load_book(&mut store, UnixSeconds(42)),
                Book::empty(),
                "{case}"
            );
            assert!(
                !file.exists(),
                "{case}: the damaged book is gone from its place"
            );
            let kept = std::fs::read(folder.join("privacy.toml.bad-42")).expect("kept aside");
            assert_eq!(kept, bytes, "{case}: and kept whole");
            std::fs::remove_file(folder.join("privacy.toml.bad-42")).expect("cleaned");
        }
    }
}
