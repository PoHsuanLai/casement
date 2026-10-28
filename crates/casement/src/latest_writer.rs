//! A file written off the loop thread, latest text wins.
//!
//! A write that is durable is a file create, a write, an fsync, a rename and an fsync of the
//! folder: on a slow or busy disk that is long enough to drop frames, and on a hung home mount it
//! never ends. [`LatestWriter`] hands the text to a thread of its own and returns at once; text
//! that arrives while a write is under way replaces the text waiting (only the latest state
//! matters), and the last text is written before the writer is dropped. A machine that cannot
//! start the thread writes on the caller's, as before.

use std::fmt::Display;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread::JoinHandle;

/// What the writer thread waits for.
#[derive(Debug, Default)]
struct Waiting {
    /// The latest text not written yet.
    text: Option<String>,
    /// The writer is gone: write what waits and end.
    closed: bool,
}

/// The slot the writer and its thread share.
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

/// How one text is written.
type WriteFn<E> = Box<dyn FnMut(&str) -> Result<(), E> + Send>;

/// A file's text written on a thread of its own.
pub struct LatestWriter<E> {
    slot: Arc<Slot>,
    thread: Option<JoinHandle<()>>,
    /// The write, shared with the thread (the caller's thread uses it when there is none).
    write: Arc<Mutex<WriteFn<E>>>,
}

impl<E> std::fmt::Debug for LatestWriter<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LatestWriter")
            .field("threaded", &self.thread.is_some())
            .finish_non_exhaustive()
    }
}

impl<E: Display + Send + 'static> LatestWriter<E> {
    /// A writer that runs `write` for each text on a thread called `name`. A failed write is
    /// logged with `what` (the file's plain name); only the caller's own thread sees an error.
    pub fn start(
        name: &str,
        what: &'static str,
        write: impl FnMut(&str) -> Result<(), E> + Send + 'static,
    ) -> LatestWriter<E> {
        let slot = Arc::new(Slot::default());
        let write: Arc<Mutex<WriteFn<E>>> = Arc::new(Mutex::new(Box::new(write)));
        let thread = {
            let (slot, write) = (Arc::clone(&slot), Arc::clone(&write));
            std::thread::Builder::new()
                .name(name.to_owned())
                .spawn(move || write_until_closed(&slot, &write, what))
                .map_err(|error| tracing::warn!(%error, what, "this file is written on the loop"))
                .ok()
        };
        LatestWriter {
            slot,
            thread,
            write,
        }
    }

    /// Hands `text` over to be written. Without a thread it is written now and the result is
    /// the write's.
    pub fn put(&self, text: &str) -> Result<(), E> {
        if self.thread.is_none() {
            let mut write = self.write.lock().unwrap_or_else(PoisonError::into_inner);
            return (*write)(text);
        }
        self.slot.lock().text = Some(text.to_owned());
        self.slot.wake.notify_one();
        Ok(())
    }
}

/// Writes the text that waits in `slot` as it comes, until the slot is closed and empty.
fn write_until_closed<E: Display>(slot: &Slot, write: &Mutex<WriteFn<E>>, what: &str) {
    loop {
        let text = {
            let mut waiting = slot.lock();
            loop {
                if let Some(text) = waiting.text.take() {
                    break text;
                }
                if waiting.closed {
                    return;
                }
                waiting = slot
                    .wake
                    .wait(waiting)
                    .unwrap_or_else(PoisonError::into_inner);
            }
        };
        let mut write = write.lock().unwrap_or_else(PoisonError::into_inner);
        if let Err(error) = (*write)(&text) {
            tracing::warn!(%error, what, "a file could not be written");
        }
    }
}

impl<E> Drop for LatestWriter<E> {
    fn drop(&mut self) {
        self.slot.lock().closed = true;
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

    /// A write that appends to a shared list, slowly enough that texts pile up behind it.
    fn recording(
        seen: &Arc<Mutex<Vec<String>>>,
    ) -> impl FnMut(&str) -> Result<(), String> + Send + use<> {
        let seen = Arc::clone(seen);
        move |text| {
            seen.lock().expect("seen").push(text.to_owned());
            Ok(())
        }
    }

    #[test]
    fn the_latest_text_is_written_before_the_writer_is_gone_and_the_caller_never_waits() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let writer = LatestWriter::start("test-writer", "a test file", recording(&seen));
        for n in 0..50 {
            writer.put(&format!("version {n}")).expect("handed over");
        }
        drop(writer);
        let seen = seen.lock().expect("seen");
        assert_eq!(seen.last().map(String::as_str), Some("version 49"));
        assert!(seen.len() <= 50, "never more writes than texts");
    }
}
