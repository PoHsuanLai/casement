//! The keymap an EIS keyboard device is told about (libei clients wait for it before they use
//! the device): the seat's keymap as text, and the sealed memory file it travels in.
//!
//! The text is the seat's (the same layout `wl_keyboard` clients get), compiled by the
//! composition root and handed in; this file only makes the file.

use rustix::fs::{MemfdFlags, SealFlags, fcntl_add_seals, memfd_create};
use std::fs::File;
use std::io::{self, Write};
use std::os::fd::OwnedFd;
use std::sync::Arc;

/// An XKB keymap in the text v1 format.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeymapText(Arc<str>);

/// A keymap in a sealed memory file, with the size to read from it.
#[derive(Debug)]
pub struct KeymapFile {
    /// The file: sealed against writes and resizing.
    pub fd: OwnedFd,
    /// The text's length in bytes, plus the NUL the format ends with.
    pub size: u32,
}

impl KeymapText {
    /// The keymap `text` (XKB text v1).
    pub fn new(text: impl Into<Arc<str>>) -> KeymapText {
        KeymapText(text.into())
    }

    /// The text.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The text in a new sealed memory file, NUL-terminated as keymap files are.
    pub fn to_file(&self) -> io::Result<KeymapFile> {
        let size = u32::try_from(self.0.len() + 1)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "keymap too large"))?;
        let fd = memfd_create(
            c"casement-eis-keymap",
            MemfdFlags::CLOEXEC | MemfdFlags::ALLOW_SEALING,
        )?;
        let mut file = File::from(fd);
        file.write_all(self.0.as_bytes())?;
        file.write_all(&[0])?;
        file.flush()?;
        let fd = OwnedFd::from(file);
        fcntl_add_seals(
            &fd,
            SealFlags::SEAL | SealFlags::SHRINK | SealFlags::GROW | SealFlags::WRITE,
        )?;
        Ok(KeymapFile { fd, size })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustix::fs::fcntl_get_seals;
    use std::io::{Read, Seek};

    /// The file holds the text and a NUL, says its size, and cannot be changed.
    #[test]
    fn the_file_holds_the_text_sealed() {
        let text = KeymapText::new("xkb_keymap { };");
        let file = text.to_file().expect("file");
        assert_eq!(usize::try_from(file.size), Ok(text.as_str().len() + 1));
        let seals = fcntl_get_seals(&file.fd).expect("seals");
        assert!(seals.contains(SealFlags::WRITE | SealFlags::GROW | SealFlags::SHRINK));
        let mut read = File::from(file.fd);
        read.rewind().expect("rewind");
        let mut bytes = Vec::new();
        read.read_to_end(&mut bytes).expect("read");
        assert_eq!(bytes, b"xkb_keymap { };\0");
    }
}
