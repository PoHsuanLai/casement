//! The administrator's floor on disk: `privacy.toml` and the `*.toml` files of `privacy.d`
//! beside it, in `/etc/quire`. casement only reads them, and believes a file only when it and
//! the folder it is in belong to the administrator alone: owned by them and not writable by
//! the group or anyone else. A file that is not trusted is reported, never read.

use super::store::{Distrust, FILE_CAP, FloorText, StoreError};
use std::fs::{File, Metadata};
use std::io::Read;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

/// The most drop-in files read.
pub const DROP_INS_MAX: usize = 64;

/// The group and other write bits.
const OTHERS_WRITE: u32 = 0o022;

/// Who must own the floor files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Owner {
    /// The administrator (user 0): every real run.
    Root,
    /// This user: a jailed run or a test, whose floor cannot be root's.
    User(u32),
}

impl Owner {
    /// The user number the files must carry.
    fn uid(self) -> u32 {
        match self {
            Owner::Root => 0,
            Owner::User(uid) => uid,
        }
    }
}

/// Where the floor's files are, and who must own them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FloorFiles {
    main: PathBuf,
    drop_ins: PathBuf,
    owner: Owner,
}

impl FloorFiles {
    /// The floor in `/etc/quire`, owned by the administrator.
    pub fn system() -> FloorFiles {
        FloorFiles::in_dir(Path::new("/etc/quire"), Owner::Root)
    }

    /// The floor in `dir` (`privacy.toml` and `privacy.d/`), whose files `owner` must own.
    pub fn in_dir(dir: &Path, owner: Owner) -> FloorFiles {
        FloorFiles {
            main: dir.join("privacy.toml"),
            drop_ins: dir.join("privacy.d"),
            owner,
        }
    }

    /// Every floor file there is: the main one, then the drop-ins by name. A file that is not
    /// there is not listed; one that is not believed is listed with why.
    pub(super) fn read(&self) -> Vec<FloorText> {
        let drop_ins = self.drop_in_paths();
        let mut found: Vec<FloorText> = Vec::new();
        found.extend(self.one(&self.main));
        match drop_ins {
            Ok(paths) => found.extend(paths.iter().filter_map(|path| self.one(path))),
            Err(error) => found.push(FloorText {
                path: self.drop_ins.clone(),
                text: Err(error),
            }),
        }
        found
    }

    /// The `*.toml` files of the drop-in folder, by name; none when the folder is not there.
    /// The folder itself is checked like a file's.
    fn drop_in_paths(&self) -> Result<Vec<PathBuf>, StoreError> {
        let entries = match std::fs::read_dir(&self.drop_ins) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(untrusted_io(&self.drop_ins, &error)),
        };
        self.closed_folder(&self.drop_ins)?;
        let mut paths: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "toml")
            })
            .collect();
        paths.sort();
        paths.truncate(DROP_INS_MAX);
        Ok(paths)
    }

    /// The file at `path`, when there is one.
    fn one(&self, path: &Path) -> Option<FloorText> {
        let opened = match File::open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return None,
            Err(error) => {
                return Some(FloorText {
                    path: path.to_path_buf(),
                    text: Err(untrusted_io(path, &error)),
                });
            }
        };
        Some(FloorText {
            path: path.to_path_buf(),
            text: self.believed_text(path, opened),
        })
    }

    /// The text of `file` (opened from `path`), when it and its folder are the administrator's
    /// alone. The file is judged by the open descriptor, so what is read is what was judged.
    fn believed_text(&self, path: &Path, file: File) -> Result<String, StoreError> {
        let link = std::fs::symlink_metadata(path).map_err(|error| untrusted_io(path, &error))?;
        let meta = file
            .metadata()
            .map_err(|error| untrusted_io(path, &error))?;
        if link.file_type().is_symlink() || !meta.is_file() {
            return Err(distrust(path, Distrust::NotPlain));
        }
        self.closed(path, &meta)?;
        if let Some(folder) = path.parent() {
            self.closed_folder(folder)?;
        }
        let mut bytes = Vec::new();
        let limit = FILE_CAP.limit();
        file.take(limit.saturating_add(1))
            .read_to_end(&mut bytes)
            .map_err(|error| untrusted_io(path, &error))?;
        if u64::try_from(bytes.len()).is_ok_and(|len| len > limit) {
            return Err(untrusted_io(
                path,
                &std::io::Error::new(std::io::ErrorKind::InvalidData, "larger than 64 KiB"),
            ));
        }
        String::from_utf8(bytes).map_err(|_| {
            untrusted_io(
                path,
                &std::io::Error::new(std::io::ErrorKind::InvalidData, "not text"),
            )
        })
    }

    fn closed_folder(&self, folder: &Path) -> Result<(), StoreError> {
        let meta = std::fs::metadata(folder).map_err(|error| untrusted_io(folder, &error))?;
        if !meta.is_dir() {
            return Err(distrust(folder, Distrust::NotPlain));
        }
        self.closed(folder, &meta)
    }

    /// Whether `meta`, of `path`, belongs to the administrator alone.
    fn closed(&self, path: &Path, meta: &Metadata) -> Result<(), StoreError> {
        if meta.uid() != self.owner.uid() {
            return Err(distrust(path, Distrust::NotTheirs));
        }
        if meta.mode() & OTHERS_WRITE != 0 {
            return Err(distrust(path, Distrust::Writable));
        }
        Ok(())
    }
}

fn distrust(path: &Path, why: Distrust) -> StoreError {
    StoreError::Untrusted {
        path: path.to_path_buf(),
        why,
    }
}

/// A file that cannot be looked at is not believed either; the reason is the system's.
fn untrusted_io(path: &Path, error: &std::io::Error) -> StoreError {
    StoreError::Io {
        path: path.to_path_buf(),
        source: std::io::Error::new(error.kind(), error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn me() -> u32 {
        rustix::process::getuid().as_raw()
    }

    fn put(path: &Path, text: &str, mode: u32) {
        std::fs::create_dir_all(path.parent().expect("a folder")).expect("a folder");
        std::fs::write(path, text).expect("a file");
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).expect("a mode");
    }

    fn mode_of(path: &Path, mode: u32) {
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).expect("a mode");
    }

    /// What reading the floor in `dir` makes of its one file: its text, or why it is not
    /// believed.
    fn only(files: &FloorFiles) -> Result<String, Distrust> {
        let mut read = files.read();
        assert_eq!(read.len(), 1, "one file: {read:?}");
        match read.remove(0).text {
            Ok(text) => Ok(text),
            Err(StoreError::Untrusted { why, .. }) => Err(why),
            Err(other) => panic!("{other}"),
        }
    }

    #[test]
    fn a_floor_file_is_believed_only_when_it_and_its_folder_are_the_administrators_alone() {
        // (case, the file's mode, who must own it, the folder's mode, what comes of it)
        let cases = [
            ("closed and theirs", 0o644, me(), 0o755, Ok(())),
            (
                "the group can write",
                0o664,
                me(),
                0o755,
                Err(Distrust::Writable),
            ),
            (
                "everyone can write",
                0o666,
                me(),
                0o755,
                Err(Distrust::Writable),
            ),
            (
                "someone else owns it",
                0o644,
                me() + 1,
                0o755,
                Err(Distrust::NotTheirs),
            ),
            (
                "the folder is open to writes",
                0o644,
                me(),
                0o777,
                Err(Distrust::Writable),
            ),
        ];
        for (case, file, owner, folder, want) in cases {
            let dir = tempfile::tempdir().expect("a scratch folder");
            let path = dir.path().join("privacy.toml");
            put(&path, "[default]\nlock = \"deny\"\n", file);
            mode_of(dir.path(), folder);
            let files = FloorFiles::in_dir(dir.path(), Owner::User(owner));
            let got = only(&files).map(|_| ());
            mode_of(dir.path(), 0o700);
            assert_eq!(got, want, "{case}");
        }
    }

    #[test]
    fn a_link_is_not_a_floor_file_and_a_missing_one_is_no_floor() {
        let dir = tempfile::tempdir().expect("a scratch folder");
        let files = FloorFiles::in_dir(dir.path(), Owner::User(me()));
        assert!(files.read().is_empty(), "nothing there");
        let real = dir.path().join("real.txt");
        put(&real, "[default]\n", 0o644);
        std::os::unix::fs::symlink(&real, dir.path().join("privacy.toml")).expect("a link");
        assert_eq!(only(&files), Err(Distrust::NotPlain));
    }

    #[test]
    fn drop_ins_are_read_by_name_and_only_the_toml_ones() {
        let dir = tempfile::tempdir().expect("a scratch folder");
        put(&dir.path().join("privacy.toml"), "# main\n", 0o644);
        put(&dir.path().join("privacy.d/20-b.toml"), "# b\n", 0o644);
        put(&dir.path().join("privacy.d/10-a.toml"), "# a\n", 0o644);
        put(
            &dir.path().join("privacy.d/notes.txt"),
            "# not a floor\n",
            0o644,
        );
        let files = FloorFiles::in_dir(dir.path(), Owner::User(me()));
        let texts: Vec<String> = files
            .read()
            .into_iter()
            .map(|file| file.text.expect("believed"))
            .collect();
        assert_eq!(texts, ["# main\n", "# a\n", "# b\n"]);
    }
}
