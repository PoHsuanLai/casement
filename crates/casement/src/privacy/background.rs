//! The person's book written off the loop thread (NEW-CASB-2). Every answer to a question and
//! every switch in Settings changes the book, and a durable write (a file create, an fsync, a
//! rename and an fsync of the folder) waits for the disk: on a busy or slow one that is long enough
//! to stall the loop with the question on the screen, and on a hung home folder it never ends.
//! [`BackgroundPrivacyStore`] reads as the file store does and writes through a [`LatestWriter`].

use crate::latest_writer::LatestWriter;
use casement_core::privilege::UnixSeconds;
use casement_privacy::{FilePrivacyStore, FloorText, PrivacyFiles, PrivacyStore, StoreError};

/// The book and the floor as files; the book is written on a thread of its own.
#[derive(Debug)]
pub struct BackgroundPrivacyStore {
    files: FilePrivacyStore,
    writer: LatestWriter<StoreError>,
}

impl BackgroundPrivacyStore {
    /// The store over `files`.
    pub fn new(files: PrivacyFiles) -> BackgroundPrivacyStore {
        let files = FilePrivacyStore::new(files);
        let sink = files.book_sink();
        let writer = LatestWriter::start("casement-privacy", "the privacy book", move |text| {
            sink.write(text)
        });
        BackgroundPrivacyStore { files, writer }
    }
}

impl PrivacyStore for BackgroundPrivacyStore {
    fn read_book(&mut self) -> Result<Option<String>, StoreError> {
        self.files.read_book()
    }

    fn set_book_aside(&mut self, at: UnixSeconds) -> Result<(), StoreError> {
        self.files.set_book_aside(at)
    }

    fn write_book(&mut self, text: &str) -> Result<(), StoreError> {
        self.writer.put(text)
    }

    fn read_floor(&mut self) -> Vec<FloorText> {
        self.files.read_floor()
    }

    fn program_exists(&self, path: &str) -> bool {
        self.files.program_exists(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use casement_privacy::FloorFiles;
    use std::os::unix::fs::MetadataExt;

    #[test]
    fn the_latest_book_is_in_a_private_file_once_the_store_is_dropped() {
        let scratch = tempfile::tempdir().expect("a scratch folder");
        let book = scratch.path().join("state/casement/privacy.toml");
        let mut store = BackgroundPrivacyStore::new(PrivacyFiles {
            book: book.clone(),
            floor: FloorFiles::system(),
        });
        for n in 0..30 {
            store
                .write_book(&format!("# version {n}\n"))
                .expect("handed over");
        }
        drop(store);
        assert_eq!(
            std::fs::read_to_string(&book).expect("written"),
            "# version 29\n"
        );
        let mode = |path: &std::path::Path| std::fs::metadata(path).expect("there").mode() & 0o777;
        assert_eq!(mode(&book), 0o600, "the file");
        assert_eq!(mode(book.parent().expect("a folder")), 0o700, "the folder");
    }
}
