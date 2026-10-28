//! The session lock's marker (CASB-06): a file that exists while the shell's lock is held.
//!
//! A compositor that restarts while the session is locked has no way to ask anyone whether it
//! was: the lock lived in memory. The marker answers. casement writes it when the shell's lock is
//! held and removes it when that lock ends; a casement that starts and finds it comes back
//! covered until the shell locks again (sill keeps its own marker for the same case and locks
//! again at once when it starts, so the two agree: both exist exactly while the shell's lock
//! is held). It lives in the runtime directory, so a reboot clears it.
//!
//! A stale marker costs one needless password prompt, never an unlocked session. This file is a
//! seam: the real one writes `$XDG_RUNTIME_DIR/casement/locked`, a test's is in memory.

use std::path::{Path, PathBuf};

/// Where the marker is kept.
pub trait LockMarker: std::fmt::Debug {
    /// Whether the marker exists now. A marker that cannot be read counts as absent.
    fn marked(&self) -> bool;

    /// Writes (`true`) or removes (`false`) the marker.
    fn set(&mut self, marked: bool) -> Result<(), LockMarkerError>;
}

/// Why the marker could not be written or removed.
#[derive(Debug, thiserror::Error)]
#[error("{}: {source}", path.display())]
pub struct LockMarkerError {
    path: PathBuf,
    #[source]
    source: std::io::Error,
}

/// No marker is kept (a run that has no runtime directory, the headless jail).
#[derive(Debug, Clone, Copy, Default)]
pub struct NoLockMarker;

impl LockMarker for NoLockMarker {
    fn marked(&self) -> bool {
        false
    }

    fn set(&mut self, _marked: bool) -> Result<(), LockMarkerError> {
        Ok(())
    }
}

/// The marker as a file.
#[derive(Debug, Clone)]
pub struct FileLockMarker {
    path: PathBuf,
}

impl FileLockMarker {
    /// The marker `locked` inside `dir` (casement's directory in the runtime directory).
    pub fn in_dir(dir: &Path) -> FileLockMarker {
        FileLockMarker {
            path: dir.join("locked"),
        }
    }

    fn failed(&self, source: std::io::Error) -> LockMarkerError {
        LockMarkerError {
            path: self.path.clone(),
            source,
        }
    }
}

impl LockMarker for FileLockMarker {
    fn marked(&self) -> bool {
        self.path.try_exists().unwrap_or(false)
    }

    fn set(&mut self, marked: bool) -> Result<(), LockMarkerError> {
        match marked {
            true => self
                .path
                .parent()
                .map_or(Ok(()), std::fs::create_dir_all)
                .and_then(|()| std::fs::write(&self.path, b"locked\n"))
                .map_err(|source| self.failed(source)),
            false => match std::fs::remove_file(&self.path) {
                Err(source) if source.kind() != std::io::ErrorKind::NotFound => {
                    Err(self.failed(source))
                }
                Ok(()) | Err(_) => Ok(()),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_marker_follows_what_was_set_and_a_missing_one_is_removed_quietly() {
        let scratch = tempfile::tempdir().expect("a scratch directory");
        let dir = scratch.path().join("casement");
        let mut marker = FileLockMarker::in_dir(&dir);
        assert!(!marker.marked());
        marker
            .set(false)
            .expect("removing what is not there is fine");
        marker.set(true).expect("written, directory made");
        assert!(marker.marked());
        assert!(
            FileLockMarker::in_dir(&dir).marked(),
            "another handle on the same directory sees it"
        );
        marker.set(false).expect("removed");
        assert!(!marker.marked());
    }

    #[test]
    fn no_marker_is_never_marked() {
        let mut none = NoLockMarker;
        none.set(true).expect("accepted");
        assert!(!none.marked());
    }
}
