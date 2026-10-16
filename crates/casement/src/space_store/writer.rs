//! The Spaces file written off the loop thread.
//!
//! A write is a file create, a write, an fsync, a rename and an fsync of the folder: on a slow
//! disk or a busy one that is long enough to drop frames, and the model asks for one on every
//! Space change. [`BackgroundSpaceStore`] hands the text to a thread of its own and returns at
//! once; text that arrives while a write is under way replaces the text waiting (only the latest
//! state matters), and the last text is written before the store is dropped.

use super::{FileSpaceStore, SpaceStore, SpaceStoreError, StateDirs, write_atomic};
use std::path::Path;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread::JoinHandle;

/// What the writer thread waits for.
#[derive(Debug, Default)]
struct Waiting {
    /// The latest text not written yet.
    text: Option<String>,
    /// The store is gone: write what waits and end.
    closed: Closed,
}

/// Whether the store was dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Closed {
    #[default]
    No,
    Yes,
}

/// The slot the store and its thread share.
#[derive(Debug, Default)]
struct Slot {
    waiting: Mutex<Waiting>,
    wake: Condvar,
}

impl Slot {
    fn lock(&self) -> MutexGuard<'_, Waiting> {
        self.waiting.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// The Spaces file in a directory, written on a thread of its own. A machine that cannot start
/// the thread writes on the caller's, as before.
#[derive(Debug)]
pub struct BackgroundSpaceStore {
    file: FileSpaceStore,
    slot: Arc<Slot>,
    thread: Option<JoinHandle<()>>,
}

impl BackgroundSpaceStore {
    /// The store for `dirs`' Spaces file.
    pub fn new(dirs: &StateDirs) -> BackgroundSpaceStore {
        let file = FileSpaceStore::new(dirs);
        let slot = Arc::new(Slot::default());
        let thread = {
            let (slot, path) = (Arc::clone(&slot), file.path.clone());
            std::thread::Builder::new()
                .name("casement-spaces".to_owned())
                .spawn(move || write_until_closed(&slot, &path))
                .map_err(|error| tracing::warn!(%error, "the Spaces file is written on the loop"))
                .ok()
        };
        BackgroundSpaceStore { file, slot, thread }
    }
}

/// Writes the text that waits in `slot` to `path` as it comes, until the slot is closed and
/// empty.
fn write_until_closed(slot: &Slot, path: &Path) {
    loop {
        let text = {
            let mut waiting = slot.lock();
            loop {
                if let Some(text) = waiting.text.take() {
                    break text;
                }
                if waiting.closed == Closed::Yes {
                    return;
                }
                waiting = slot
                    .wake
                    .wait(waiting)
                    .unwrap_or_else(PoisonError::into_inner);
            }
        };
        if let Err(error) = write_atomic(path, &text) {
            tracing::warn!(%error, "the Spaces file could not be written");
        }
    }
}

impl SpaceStore for BackgroundSpaceStore {
    fn read(&self) -> Result<Option<String>, SpaceStoreError> {
        self.file.read()
    }

    fn write(&mut self, text: &str) -> Result<(), SpaceStoreError> {
        if self.thread.is_none() {
            return write_atomic(&self.file.path, text);
        }
        self.slot.lock().text = Some(text.to_owned());
        self.slot.wake.notify_one();
        Ok(())
    }
}

impl Drop for BackgroundSpaceStore {
    fn drop(&mut self) {
        self.slot.lock().closed = Closed::Yes;
        self.slot.wake.notify_one();
        if let Some(thread) = self.thread.take() {
            // The thread writes the last text first; a panic in it is not ours to carry on.
            let _ = thread.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_latest_text_is_on_disk_once_the_store_is_dropped() {
        let dir = tempfile::tempdir().expect("a scratch folder");
        let dirs = StateDirs::at(dir.path().join("casement"));
        let mut store = BackgroundSpaceStore::new(&dirs);
        for n in 0..50 {
            store
                .write(&format!("version = {n}\n"))
                .expect("handed over");
        }
        drop(store);
        let got = std::fs::read_to_string(dirs.spaces_file()).expect("written");
        assert_eq!(got, "version = 49\n");
        let left: Vec<_> = std::fs::read_dir(dirs.dir())
            .expect("the folder")
            .filter_map(Result::ok)
            .map(|entry| entry.file_name())
            .collect();
        assert_eq!(left, [std::ffi::OsString::from("spaces.toml")]);
    }

    #[test]
    fn reading_sees_the_file_and_a_missing_one_is_nothing() {
        let dir = tempfile::tempdir().expect("a scratch folder");
        let dirs = StateDirs::at(dir.path().join("casement"));
        let store = BackgroundSpaceStore::new(&dirs);
        assert_eq!(store.read().expect("no file is not an error"), None);
    }
}
