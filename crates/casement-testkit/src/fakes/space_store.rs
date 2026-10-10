//! The fake Spaces file: `MemorySpaceStore` keeps the text in memory behind a shared handle, so
//! a test holds one clone while the `State` holds another, and reads back what the compositor
//! stored (A1). A second `State` started on a clone of the same store is a restart.

use casement::space_store::{SpaceStore, SpaceStoreError};
use std::sync::{Arc, Mutex, PoisonError};

/// What the store holds and how often it was written.
#[derive(Debug, Default)]
struct Inner {
    text: Option<String>,
    writes: usize,
}

/// A Spaces file in memory.
#[derive(Debug, Clone, Default)]
pub struct MemorySpaceStore {
    inner: Arc<Mutex<Inner>>,
}

/// Two handles are equal when they are the same store.
impl PartialEq for MemorySpaceStore {
    fn eq(&self, other: &MemorySpaceStore) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
}

impl Eq for MemorySpaceStore {}

impl MemorySpaceStore {
    /// A store with no file yet.
    pub fn empty() -> MemorySpaceStore {
        MemorySpaceStore::default()
    }

    /// A store already holding `text`.
    pub fn holding(text: &str) -> MemorySpaceStore {
        let store = MemorySpaceStore::default();
        store.lock().text = Some(text.to_owned());
        store
    }

    /// The stored text, if any was stored.
    pub fn stored(&self) -> Option<String> {
        self.lock().text.clone()
    }

    /// How many times the text was written.
    pub fn writes(&self) -> usize {
        self.lock().writes
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl SpaceStore for MemorySpaceStore {
    fn read(&self) -> Result<Option<String>, SpaceStoreError> {
        Ok(self.stored())
    }

    fn write(&mut self, text: &str) -> Result<(), SpaceStoreError> {
        let mut inner = self.lock();
        inner.text = Some(text.to_owned());
        inner.writes += 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_clone_sees_what_another_wrote() {
        let mut writer = MemorySpaceStore::empty();
        let reader = writer.clone();
        assert_eq!(reader.read().expect("read"), None);
        writer.write("spaces = []").expect("write");
        assert_eq!(reader.read().expect("read").as_deref(), Some("spaces = []"));
        assert_eq!(reader.writes(), 1);
    }
}
