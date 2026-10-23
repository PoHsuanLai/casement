//! Inhibitor pipes: the client gets the write end, the fake watches the read end, and the
//! client closing its end is the release (logind's contract for `Inhibit`).

use super::LogindError;
use super::log::Recorder;
use rustix::pipe::{PipeFlags, pipe_with};
use std::os::fd::OwnedFd;
use tokio::io::AsyncReadExt;
use tokio::net::unix::pipe::Receiver;
use tokio::runtime::Handle;

/// A close-on-exec pipe: (read end, write end).
pub(super) fn pipe() -> Result<(OwnedFd, OwnedFd), LogindError> {
    pipe_with(PipeFlags::CLOEXEC).map_err(|source| LogindError::Pipe { source })
}

/// Records an inhibitor for `what`, starts watching it on `handle` and returns the descriptor
/// the client holds.
pub(super) fn open(log: &Recorder, handle: &Handle, what: &str) -> Result<OwnedFd, LogindError> {
    let (read, write) = pipe()?;
    let id = log.hold(what);
    handle.spawn(watch(read, log.clone(), id));
    Ok(write)
}

/// Waits for the write end to close (end of file on the read end), then records the release.
async fn watch(read: OwnedFd, log: Recorder, id: u64) {
    if let Ok(mut receiver) = Receiver::from_owned_fd(read) {
        let mut buf = [0u8; 16];
        while let Ok(1..) = receiver.read(&mut buf).await {}
    }
    log.release(id);
}
