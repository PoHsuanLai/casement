//! The Space ids file's I/O seam (A1, P1-Z; filled by P1-M2).
//!
//! The codec is pure and lives in core (`casement_core::space::SpaceFile`); reading and writing
//! `$XDG_STATE_HOME/casement/spaces.toml` is this seam. The path comes from an injected
//! [`StateDirs`] that `main` resolves from the environment; the library reads none. A write is
//! atomic (a temporary file in the same directory, then a rename), so a crash never leaves half
//! a file. `casement-testkit` exports `MemorySpaceStore`.
//!
//! ```no_run
//! use casement::space_store::{FileSpaceStore, StateDirs, load_spaces};
//! use std::path::PathBuf;
//!
//! let dirs = StateDirs::at(PathBuf::from("/tmp/state/casement"));
//! let store = FileSpaceStore::new(&dirs);
//! let file = load_spaces(&store).expect("a missing file is no Spaces");
//! assert!(file.spaces.is_empty());
//! ```

mod writer;

pub use writer::BackgroundSpaceStore;

use casement_core::space::{SpaceFile, SpaceFileError};
use std::io::Write;
use std::path::{Path, PathBuf};

/// The directory casement keeps its state files in (`$XDG_STATE_HOME/casement`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StateDirs {
    casement: PathBuf,
    /// casement's directory in the session's runtime directory (the session lock's marker
    /// lives there, so a reboot clears it); none keeps no marker.
    runtime: Option<PathBuf>,
}

impl StateDirs {
    /// State kept in `dir`.
    pub fn at(dir: PathBuf) -> StateDirs {
        StateDirs {
            casement: dir,
            runtime: None,
        }
    }

    /// The same, keeping the session lock's marker in `dir`.
    #[must_use]
    pub fn with_runtime(self, dir: PathBuf) -> StateDirs {
        StateDirs {
            runtime: Some(dir),
            ..self
        }
    }

    /// Where the session lock's marker is kept, if it is.
    pub fn runtime_dir(&self) -> Option<&Path> {
        self.runtime.as_deref()
    }

    /// The directory.
    pub fn dir(&self) -> &Path {
        &self.casement
    }

    /// The Space ids file (C52).
    pub fn spaces_file(&self) -> PathBuf {
        self.casement.join("spaces.toml")
    }
}

/// Where casement keeps state: nowhere (tests, the headless jail), or in a directory.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum StateHome {
    /// Nothing is read or kept between runs.
    Ephemeral,
    /// State lives here.
    At(StateDirs),
}

/// Why the Spaces file could not be read or written.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SpaceStoreError {
    /// The file system refused.
    #[error("{}: {source}", path.display())]
    Io {
        /// The file.
        path: PathBuf,
        /// Why.
        #[source]
        source: std::io::Error,
    },
    /// The text did not parse or render.
    #[error("{0}")]
    Codec(#[from] SpaceFileError),
}

/// The Spaces file's text, stored somewhere.
pub trait SpaceStore: std::fmt::Debug {
    /// The stored text, or none when nothing was stored yet.
    fn read(&self) -> Result<Option<String>, SpaceStoreError>;

    /// Stores `text`, replacing what was there, atomically.
    fn write(&mut self, text: &str) -> Result<(), SpaceStoreError>;
}

/// The Spaces file in a directory.
#[derive(Debug)]
pub struct FileSpaceStore {
    /// The file.
    pub path: PathBuf,
}

impl FileSpaceStore {
    /// The store for `dirs`' Spaces file.
    pub fn new(dirs: &StateDirs) -> FileSpaceStore {
        FileSpaceStore {
            path: dirs.spaces_file(),
        }
    }
}

impl SpaceStore for FileSpaceStore {
    fn read(&self) -> Result<Option<String>, SpaceStoreError> {
        match std::fs::read_to_string(&self.path) {
            Ok(text) => Ok(Some(text)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(source) => Err(SpaceStoreError::Io {
                path: self.path.clone(),
                source,
            }),
        }
    }

    fn write(&mut self, text: &str) -> Result<(), SpaceStoreError> {
        write_atomic(&self.path, text)
    }
}

/// Writes `text` to `path` atomically and durably: a temporary file in the same folder, synced,
/// renamed over `path`, and the folder synced, so a crash or a power cut leaves the old file or
/// the new one and never half of either.
pub(crate) fn write_atomic(path: &Path, text: &str) -> Result<(), SpaceStoreError> {
    let io = |at: &Path| {
        let path = at.to_path_buf();
        move |source| SpaceStoreError::Io { path, source }
    };
    let dir = path.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(dir).map_err(io(dir))?;
    let temporary = path.with_extension("toml.tmp");
    let written = (|| {
        let mut file = std::fs::File::create(&temporary)?;
        file.write_all(text.as_bytes())?;
        file.sync_all()
    })();
    written.map_err(io(&temporary))?;
    std::fs::rename(&temporary, path).map_err(|source| {
        let _ = std::fs::remove_file(&temporary);
        SpaceStoreError::Io {
            path: path.to_path_buf(),
            source,
        }
    })?;
    // The rename is in the folder's entry: sync the folder so it survives a power cut.
    std::fs::File::open(dir)
        .and_then(|folder| folder.sync_all())
        .map_err(io(dir))
}

/// A store that keeps nothing: reads find no file, writes are dropped.
#[derive(Debug, Default)]
pub struct EphemeralSpaceStore;

impl SpaceStore for EphemeralSpaceStore {
    fn read(&self) -> Result<Option<String>, SpaceStoreError> {
        Ok(None)
    }

    fn write(&mut self, text: &str) -> Result<(), SpaceStoreError> {
        let _ = text;
        Ok(())
    }
}

/// The stored Spaces: none when nothing was stored.
pub fn load_spaces(store: &dyn SpaceStore) -> Result<SpaceFile, SpaceStoreError> {
    match store.read()? {
        Some(text) => Ok(SpaceFile::parse(&text)?),
        None => Ok(SpaceFile::empty()),
    }
}

/// Stores `file`.
pub fn save_spaces(store: &mut dyn SpaceStore, file: &SpaceFile) -> Result<(), SpaceStoreError> {
    store.write(&file.render()?)
}
