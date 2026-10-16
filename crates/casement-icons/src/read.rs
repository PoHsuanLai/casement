//! The one way a file is read here: a plain file only, never more than a cap.
//!
//! An icon path can come from a client (through its `xdg_toplevel_icon` name, once it is a theme
//! name) or from a desktop entry a person can edit, so it may name `/dev/zero`, a pipe nobody
//! writes to, or a file that grows without end. `std::fs::read` would block or fill memory on
//! each of them.

use crate::IconError;
use rustix::fs::{Mode, OFlags, open};
use std::fs::File;
use std::io::Read;
use std::path::Path;

/// The bytes of the plain file at `path`, at most `limit` of them.
///
/// The file is opened without blocking (a pipe with no writer opens at once), looked at through
/// the open descriptor (so a path swapped for a device between a check and the read is no gap),
/// and refused unless it is a regular file. Symbolic links are followed: icon themes are full of
/// them, and what counts is what they lead to. A file longer than `limit` is refused, not cut.
pub(crate) fn plain_file(path: &Path, limit: u64) -> Result<Vec<u8>, IconError> {
    let read = |source: std::io::Error| IconError::Read {
        path: path.to_path_buf(),
        source,
    };
    let fd = open(
        path,
        OFlags::RDONLY | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|errno| read(std::io::Error::from(errno)))?;
    let file = File::from(fd);
    let metadata = file.metadata().map_err(read)?;
    if !metadata.is_file() {
        return Err(IconError::NotAFile {
            path: path.to_path_buf(),
        });
    }
    let mut bytes = Vec::new();
    // One byte past the limit tells a file of exactly `limit` bytes from a longer one.
    file.take(limit.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(read)?;
    match u64::try_from(bytes.len()).is_ok_and(|len| len <= limit) {
        true => Ok(bytes),
        false => Err(IconError::TooLarge {
            path: path.to_path_buf(),
        }),
    }
}
