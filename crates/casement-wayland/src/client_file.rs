//! Reading a file a client handed over (a virtual keyboard's keymap, a gamma ramp) on the
//! loop (CASW-09). A client can pass any file descriptor it likes, and a regular file on a
//! filesystem it serves itself (a FUSE mount a user may make) blocks a `pread` for as long as
//! its process wants: the whole session would stand still. So only a memory file is read: a
//! `memfd`, or a file on `tmpfs`, which is what the protocols ask clients to send anyway.
//!
//! The test is `fcntl(F_GET_SEALS)`: it succeeds on exactly those files and fails (`EINVAL`) on
//! a pipe, a device, a socket and a file of any other filesystem, and it asks the filesystem
//! nothing, so it cannot block on a hostile one the way `fstat` or `fstatfs` could.

use smithay::reexports::rustix::fs::fcntl_get_seals;
use std::fs::File;
use std::os::unix::fs::FileExt;

/// Whether `file` is a memory file (a `memfd` or a `tmpfs` file): one whose reads never wait on
/// another process.
pub fn is_memory(file: &File) -> bool {
    fcntl_get_seals(file).is_ok()
}

/// The first `length` bytes of `file`, when it is a memory file and holds them; none otherwise.
pub fn read_exactly(file: &File, length: usize) -> Option<Vec<u8>> {
    if !is_memory(file) {
        return None;
    }
    let mut bytes = vec![0; length];
    file.read_exact_at(&mut bytes, 0).ok()?;
    Some(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use smithay::reexports::rustix::fs::{MemfdFlags, memfd_create};
    use smithay::reexports::rustix::pipe::pipe;
    use std::io::Write;

    fn memory_file(content: &[u8]) -> File {
        let mut file = File::from(memfd_create("test", MemfdFlags::CLOEXEC).expect("a memfd"));
        file.write_all(content).expect("write");
        file
    }

    /// The bytes of a memory file are read; a short file is not.
    #[test]
    fn a_memory_file_is_read_exactly() {
        let file = memory_file(b"keymap");
        assert_eq!(read_exactly(&file, 6), Some(b"keymap".to_vec()));
        assert_eq!(read_exactly(&file, 3), Some(b"key".to_vec()));
        assert_eq!(read_exactly(&file, 7), None, "the file is shorter");
    }

    /// CASW-09: what is not a memory file is not read: a pipe, and a regular file of a
    /// filesystem that is not tmpfs (`/proc/self/stat` stands in for a FUSE file: readable, and
    /// asked of its filesystem on every read).
    #[test]
    fn anything_else_is_not_read() {
        let (reader, _writer) = pipe().expect("a pipe");
        let pipe_end = File::from(reader);
        assert!(!is_memory(&pipe_end));
        assert_eq!(read_exactly(&pipe_end, 1), None);
        let other = File::open("/proc/self/stat").expect("procfs");
        assert!(!is_memory(&other));
        assert_eq!(read_exactly(&other, 1), None);
    }
}
