//! The X11 listening sockets casement holds while satellite is not running (P1-X1).
//!
//! A display is a pair of sockets: the path socket `<dir>/X<n>` and the abstract socket
//! `@<dir>/X<n>` (what `libxcb` tries first). casement binds both, so a client that connects
//! finds a server before satellite exists; the connection waits in the backlog until satellite
//! is started with the same descriptors and accepts it.

use super::{DisplayName, Restarts, XwaylandError};
use rustix::event::{PollFd, PollFlags, Timespec, poll};
use rustix::net::{
    AddressFamily, SocketAddrUnix, SocketFlags, SocketType, bind, listen, socket_with,
};
use std::os::fd::{AsFd, OwnedFd};
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};

/// The directory X11 path sockets live in (`/tmp/.X11-unix` on a real system).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct X11Dir(pub PathBuf);

/// The highest display number tried.
const LAST_DISPLAY: u32 = 63;

/// The backlog of each socket.
const BACKLOG: i32 = 128;

/// The bound sockets of one display.
#[derive(Debug)]
pub struct Listening {
    display: DisplayName,
    path: PathBuf,
    sockets: Vec<OwnedFd>,
    restarts: Restarts,
}

impl Listening {
    /// Binds the first free display in `dir` (created when missing).
    ///
    /// ```no_run
    /// use casement::xwayland::{Listening, X11Dir};
    ///
    /// let dir = std::env::temp_dir().join(format!("casement-doc-x11-{}", std::process::id()));
    /// let listening = Listening::bind(&X11Dir(dir.clone())).expect("a free display");
    /// assert!(listening.display().0.starts_with(':'));
    /// drop(listening);
    /// let _ = std::fs::remove_dir_all(dir);
    /// ```
    pub fn bind(dir: &X11Dir) -> Result<Listening, XwaylandError> {
        std::fs::create_dir_all(&dir.0)?;
        check_dir(&dir.0)?;
        (0..=LAST_DISPLAY)
            .find_map(|number| try_display(&dir.0, number))
            .ok_or(XwaylandError::NoDisplay)
    }

    /// The display these sockets serve.
    pub fn display(&self) -> &DisplayName {
        &self.display
    }

    /// The recent satellite starts on this display.
    pub(super) fn restarts_mut(&mut self) -> &mut Restarts {
        &mut self.restarts
    }

    /// The sockets, for registering with the event loop.
    pub fn sockets(&self) -> &[OwnedFd] {
        &self.sockets
    }

    /// Whether a client is waiting to be accepted on any of the sockets.
    pub fn connection_waiting(&self) -> bool {
        let mut fds: Vec<PollFd<'_>> = self
            .sockets
            .iter()
            .map(|fd| PollFd::new(fd, PollFlags::IN))
            .collect();
        let now = Timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        matches!(poll(&mut fds, Some(&now)), Ok(n) if n > 0)
    }

    /// Copies of the sockets, to hand to a program while casement keeps its own.
    pub fn duplicate(&self) -> Result<Vec<OwnedFd>, XwaylandError> {
        self.sockets
            .iter()
            .map(|fd| {
                fd.as_fd()
                    .try_clone_to_owned()
                    .map_err(XwaylandError::Socket)
            })
            .collect()
    }
}

impl Drop for Listening {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Refuses a folder other people could put sockets in: it must be a real folder (not a link)
/// owned by root or by us, and not writable by others unless sticky (as `/tmp/.X11-unix` is,
/// mode 1777): in a folder anyone may write, another user could have made the socket we are
/// about to take, or swap it under us.
fn check_dir(dir: &Path) -> Result<(), XwaylandError> {
    let meta = std::fs::symlink_metadata(dir)?;
    let owner_ok = meta.uid() == 0 || meta.uid() == rustix::process::getuid().as_raw();
    let others_write = meta.mode() & 0o022 != 0;
    let sticky = meta.mode() & 0o1000 != 0;
    match meta.is_dir() && owner_ok && (!others_write || sticky) {
        true => Ok(()),
        false => Err(XwaylandError::Socket(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            format!("{} is not a safe place for sockets", dir.display()),
        ))),
    }
}

/// Removes the socket at `path` when it is a leftover: a socket file that nothing listens on
/// (a casement that was killed leaves its sockets behind), so that display number is taken
/// again instead of the next one.
fn clear_leftover(path: &Path) {
    let is_socket = std::fs::symlink_metadata(path).is_ok_and(|meta| meta.file_type().is_socket());
    let refused = matches!(
        UnixStream::connect(path),
        Err(error) if error.kind() == std::io::ErrorKind::ConnectionRefused
    );
    if is_socket && refused {
        let _ = std::fs::remove_file(path);
    }
}

/// Binds display `number` in `dir`, or `None` when it is taken.
fn try_display(dir: &Path, number: u32) -> Option<Listening> {
    let path = dir.join(format!("X{number}"));
    clear_leftover(&path);
    let listener = UnixListener::bind(&path).ok()?;
    let Some(abstract_socket) = bind_abstract(&path) else {
        let _ = std::fs::remove_file(&path);
        return None;
    };
    Some(Listening {
        display: DisplayName(format!(":{number}")),
        path,
        sockets: vec![OwnedFd::from(listener), abstract_socket],
        restarts: Restarts::default(),
    })
}

/// The abstract socket named like `path`.
fn bind_abstract(path: &Path) -> Option<OwnedFd> {
    let name = path.to_str()?.as_bytes();
    let address = SocketAddrUnix::new_abstract_name(name).ok()?;
    let fd = socket_with(
        AddressFamily::UNIX,
        SocketType::STREAM,
        SocketFlags::CLOEXEC,
        None,
    )
    .ok()?;
    bind(&fd, &address).ok()?;
    listen(&fd, BACKLOG).ok()?;
    Some(fd)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn a_display_left_by_a_killed_run_is_taken_again() {
        let dir = tempfile::tempdir().expect("a scratch folder");
        // A killed run closes its sockets but removes no file: the path stays, nobody listens.
        let leftover = dir.path().join("X0");
        drop(UnixListener::bind(&leftover).expect("a socket file"));
        assert!(leftover.exists());
        let again = Listening::bind(&X11Dir(dir.path().to_owned())).expect("a free display");
        assert_eq!(again.display(), &DisplayName(":0".to_owned()));
    }

    #[test]
    fn a_display_someone_listens_on_is_not_taken() {
        let dir = tempfile::tempdir().expect("a scratch folder");
        let first = Listening::bind(&X11Dir(dir.path().to_owned())).expect("a free display");
        let second = Listening::bind(&X11Dir(dir.path().to_owned())).expect("the next one");
        assert_ne!(first.display(), second.display());
    }

    #[test]
    fn a_folder_anyone_may_write_in_is_refused_unless_sticky() {
        let dir = tempfile::tempdir().expect("a scratch folder");
        let open = dir.path().join("open");
        std::fs::create_dir(&open).expect("folder");
        // name, mode, accepted
        let cases = [
            ("private", 0o700, true),
            ("group and others may read", 0o755, true),
            ("others may write", 0o777, false),
            ("others may write, sticky", 0o1777, true),
        ];
        for (name, mode, accepted) in cases {
            std::fs::set_permissions(&open, std::fs::Permissions::from_mode(mode)).expect("mode");
            assert_eq!(check_dir(&open).is_ok(), accepted, "{name}");
        }
    }
}
