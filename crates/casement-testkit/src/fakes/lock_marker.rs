//! The fake session-lock marker: `MemoryLockMarker` keeps the flag in memory behind a shared
//! handle, so a test holds one clone while the `State` holds another, and a second `State`
//! started on a clone of the same marker is a restart while locked.

use casement::lock_marker::{LockMarker, LockMarkerError, Marked};
use std::sync::{Arc, Mutex, PoisonError};

/// What the marker holds and how often it was written.
#[derive(Debug)]
struct Inner {
    marked: Marked,
    writes: usize,
}

/// A lock marker in memory.
#[derive(Debug, Clone)]
pub struct MemoryLockMarker {
    inner: Arc<Mutex<Inner>>,
}

/// Two handles are equal when they are the same marker.
impl PartialEq for MemoryLockMarker {
    fn eq(&self, other: &MemoryLockMarker) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
}

impl Eq for MemoryLockMarker {}

impl Default for MemoryLockMarker {
    fn default() -> MemoryLockMarker {
        MemoryLockMarker::absent()
    }
}

impl MemoryLockMarker {
    /// A marker that does not exist.
    pub fn absent() -> MemoryLockMarker {
        MemoryLockMarker::holding(Marked::No)
    }

    /// A marker as `marked` says.
    pub fn holding(marked: Marked) -> MemoryLockMarker {
        MemoryLockMarker {
            inner: Arc::new(Mutex::new(Inner { marked, writes: 0 })),
        }
    }

    /// Whether the marker exists now.
    pub fn is_marked(&self) -> Marked {
        self.lock().marked
    }

    /// How many times the marker was written or removed.
    pub fn writes(&self) -> usize {
        self.lock().writes
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl LockMarker for MemoryLockMarker {
    fn marked(&self) -> Marked {
        self.is_marked()
    }

    fn set(&mut self, marked: Marked) -> Result<(), LockMarkerError> {
        let mut inner = self.lock();
        inner.marked = marked;
        inner.writes += 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_clone_sees_what_another_set() {
        let mut writer = MemoryLockMarker::absent();
        let reader = writer.clone();
        assert_eq!(reader.is_marked(), Marked::No);
        writer.set(Marked::Yes).expect("set");
        assert_eq!(reader.is_marked(), Marked::Yes);
        assert_eq!(reader.writes(), 1);
    }
}
