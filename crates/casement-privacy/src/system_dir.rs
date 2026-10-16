//! Where the administrator's files are looked for.

use super::floor_files::{FloorFiles, Owner};
use std::path::PathBuf;

/// Where the administrator's files are looked for.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum SystemDir {
    /// `/etc/quire`, owned by the administrator.
    #[default]
    Etc,
    /// This folder instead (`--system-dir`): a jailed run or a test whose floor is its own
    /// user's.
    At(PathBuf),
}

impl SystemDir {
    /// The floor's files. A floor in an injected folder is owned by `injected` (the user, in a
    /// build for the jail; the administrator anywhere else).
    pub fn floor(&self, injected: Owner) -> FloorFiles {
        match self {
            SystemDir::Etc => FloorFiles::system(),
            SystemDir::At(dir) => FloorFiles::in_dir(dir, injected),
        }
    }
}
