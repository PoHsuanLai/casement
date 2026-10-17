//! The Spaces file written off the loop thread: the model asks for a write on every Space
//! change, and a durable write is long enough on a busy disk to drop frames
//! ([`LatestWriter`]: the latest text wins, the last is written before the store is dropped).

use super::{FileSpaceStore, SpaceStore, SpaceStoreError, StateDirs, write_atomic};
use crate::latest_writer::LatestWriter;

/// The Spaces file in a directory, written on a thread of its own ([`LatestWriter`]).
#[derive(Debug)]
pub struct BackgroundSpaceStore {
    file: FileSpaceStore,
    writer: LatestWriter<SpaceStoreError>,
}

impl BackgroundSpaceStore {
    /// The store for `dirs`' Spaces file.
    pub fn new(dirs: &StateDirs) -> BackgroundSpaceStore {
        let file = FileSpaceStore::new(dirs);
        let path = file.path.clone();
        let writer = LatestWriter::start("casement-spaces", "the Spaces file", move |text| {
            write_atomic(&path, text)
        });
        BackgroundSpaceStore { file, writer }
    }
}

impl SpaceStore for BackgroundSpaceStore {
    fn read(&self) -> Result<Option<String>, SpaceStoreError> {
        self.file.read()
    }

    fn write(&mut self, text: &str) -> Result<(), SpaceStoreError> {
        self.writer.put(text)
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
