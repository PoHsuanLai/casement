//! Inhibitor pipes: the client gets the write end, the fake watches the read end, and the
//! client closing its end is the release (logind's contract for `Inhibit`).

use super::LogindError;
use super::log::Recorder;
use rustix::fs::{OFlags, fcntl_setfl};
use rustix::io::Errno;
use rustix::pipe::{PipeFlags, pipe_with};
use std::os::fd::OwnedFd;
use tokio::io::unix::AsyncFd;
use tokio::runtime::Handle;

/// A close-on-exec pipe: (read end, write end).
pub(super) fn pipe() -> Result<(OwnedFd, OwnedFd), LogindError> {
    pipe_with(PipeFlags::CLOEXEC).map_err(|source| LogindError::Pipe { source })
}

/// Records an inhibitor for `what`, starts watching it on `handle` and returns the descriptor
/// the client holds.
pub(super) fn open(log: &Recorder, handle: &Handle, what: &str) -> Result<OwnedFd, LogindError> {
    let (read, write) = pipe()?;
    fcntl_setfl(&read, OFlags::NONBLOCK).map_err(|source| LogindError::Pipe { source })?;
    let id = log.hold(what);
    handle.spawn(watch(read, log.clone(), id));
    Ok(write)
}

/// Waits for the write end to close (end of file on the read end), then records the release.
async fn watch(read: OwnedFd, log: Recorder, id: u64) {
    if let Ok(ready) = AsyncFd::new(read) {
        let mut buf = [0u8; 16];
        loop {
            let Ok(mut guard) = ready.readable().await else {
                break;
            };
            match rustix::io::read(ready.get_ref(), &mut buf) {
                Ok(0) => break,
                Ok(_) => {}
                Err(error) if error == Errno::AGAIN => guard.clear_ready(),
                Err(_) => break,
            }
        }
    }
    log.release(id);
}
