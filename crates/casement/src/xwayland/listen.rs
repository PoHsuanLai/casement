//! The X11 listening sockets casement holds while satellite is not running (P1-X1).
//!
//! A display is a pair of sockets: the path socket `<dir>/X<n>` and the abstract socket
//! `@<dir>/X<n>` (what `libxcb` tries first). casement binds both, so a client that connects
//! finds a server before satellite exists; the connection waits in the backlog until satellite
//! is started with the same descriptors and accepts it.

use super::{DisplayName, XwaylandError};
use rustix::event::{PollFd, PollFlags, Timespec, poll};
use rustix::net::{
    AddressFamily, SocketAddrUnix, SocketFlags, SocketType, bind, listen, socket_with,
};
use std::os::fd::{AsFd, OwnedFd};
use std::os::unix::net::UnixListener;
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
        (0..=LAST_DISPLAY)
            .find_map(|number| try_display(&dir.0, number))
            .ok_or(XwaylandError::NoDisplay)
    }

    /// The display these sockets serve.
    pub fn display(&self) -> &DisplayName {
        &self.display
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

/// Binds display `number` in `dir`, or `None` when it is taken.
fn try_display(dir: &Path, number: u32) -> Option<Listening> {
    let path = dir.join(format!("X{number}"));
    let listener = UnixListener::bind(&path).ok()?;
    let Some(abstract_socket) = bind_abstract(&path) else {
        let _ = std::fs::remove_file(&path);
        return None;
    };
    Some(Listening {
        display: DisplayName(format!(":{number}")),
        path,
        sockets: vec![OwnedFd::from(listener), abstract_socket],
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
