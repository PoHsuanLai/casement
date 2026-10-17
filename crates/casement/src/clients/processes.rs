//! The process behind each public connection, read from the socket's peer credentials when the
//! client connects. Force quit signals these (`exec/quit.rs`); the display's own client list is
//! never walked for it. xwayland-satellite connects through the public socket like any app, but
//! it carries every X11 program's windows and is not an app's process: it is never recorded as
//! one. Its connection is remembered as an X11 carrier, so force quit knows to ask the X server
//! for the processes behind the windows (`xwayland::owners`).

use casement_core::ClientKey;
use casement_core::window::WindowOrigin;
use casement_ipc::dbus::peer::Alive;
use nix::errno::Errno;
use nix::sys::socket::{getsockopt, sockopt::PeerPidfd};
use rustix::event::{PollFd, PollFlags, Timespec, poll};
use rustix::process::{
    Pid, PidfdFlags, RawPid, Signal, kill_process, pidfd_open, pidfd_send_signal,
};
use std::collections::{BTreeMap, BTreeSet};
use std::os::fd::OwnedFd;
use std::os::unix::net::UnixStream;

/// An app's process behind a connection: its pid, and a handle on that very process when one
/// could be opened at accept (CASB-20). A signal sent through the handle can only reach the
/// process the connection had; a signal by pid, minutes later, can reach whatever took the
/// number after it exited.
#[derive(Debug)]
struct Held {
    pid: RawPid,
    handle: Option<OwnedFd>,
}

/// What is on the other end of a public connection.
#[derive(Debug)]
enum Peer {
    /// An app's own process.
    App(Held),
    /// xwayland-satellite, whose one connection carries every X11 program's windows.
    Satellite,
}

/// A handle on the process behind `stream`: the kernel's own, made when the client connected
/// (`SO_PEERPIDFD`, Linux 6.5), so it is the connecting process and never one that took its pid
/// after it exited (NEW-CASB-7). A connector that has exited has none. On a kernel without the
/// option, the pid is opened and kept only when the socket still names it ([`reopened`]).
pub(crate) fn handle_of(stream: &UnixStream, pid: RawPid) -> Option<OwnedFd> {
    match getsockopt(stream, PeerPidfd) {
        Ok(handle) => Some(handle),
        Err(Errno::ESRCH) => None,
        Err(_) => reopened(stream, pid),
    }
}

/// A handle on the process `pid` behind `stream`, when it is the one the socket names now as well
/// (a pid read from the socket and opened later could be another process that took the number).
fn reopened(stream: &UnixStream, pid: RawPid) -> Option<OwnedFd> {
    let handle = pidfd_open(Pid::from_raw(pid)?, PidfdFlags::empty()).ok()?;
    let again = rustix::net::sockopt::socket_peercred(stream).ok()?;
    (again.pid.as_raw_pid() == pid).then_some(handle)
}

/// Whether the process `handle` holds still runs: a pidfd turns readable when its process exits.
/// Facts read from `/proc/<pid>` before this says [`Alive::Yes`] belong to that process.
pub(crate) fn alive(handle: &OwnedFd) -> Alive {
    let mut fds = [PollFd::new(handle, PollFlags::IN)];
    match poll(&mut fds, Some(&Timespec::default())) {
        Ok(0) => Alive::Yes,
        _ => Alive::Gone,
    }
}

/// Sends `signal` to the process `held` names; whether it arrived.
fn reach(held: &Held, signal: Signal) -> bool {
    match &held.handle {
        Some(handle) => pidfd_send_signal(handle, signal).is_ok(),
        None => Pid::from_raw(held.pid).is_some_and(|pid| kill_process(pid, signal).is_ok()),
    }
}

/// The process of each public connection still open.
#[derive(Debug, Default)]
pub struct Processes(BTreeMap<ClientKey, Peer>);

impl Processes {
    /// Records the process on the other end of `stream` as `client`'s, when the kernel names
    /// one. When it is `satellite` (the running xwayland-satellite, whose connection serves
    /// every X11 window and whose process force quit must never signal), the connection is
    /// recorded as an X11 carrier and no process is recorded.
    pub fn record(&mut self, client: ClientKey, stream: &UnixStream, satellite: Option<RawPid>) {
        let peer = rustix::net::sockopt::socket_peercred(stream)
            .ok()
            .map(|peer| peer.pid.as_raw_pid())
            .map(|pid| match Some(pid) == satellite {
                true => Peer::Satellite,
                false => Peer::App(Held {
                    pid,
                    handle: handle_of(stream, pid),
                }),
            });
        if let Some(peer) = peer {
            self.0.insert(client, peer);
        }
    }

    /// Drops `client`'s process: its connection closed.
    pub fn forget(&mut self, client: ClientKey) {
        self.0.remove(&client);
    }

    /// The recorded processes of `clients` (an X11 carrier's connection has none).
    pub fn of(&self, clients: &[ClientKey]) -> impl Iterator<Item = RawPid> {
        clients
            .iter()
            .filter_map(|client| match self.0.get(client) {
                Some(Peer::App(held)) => Some(held.pid),
                Some(Peer::Satellite) | None => None,
            })
    }

    /// Sends `signal` to the processes of `clients`, except init, `own` and `spared` (the
    /// satellite): through the handle held since the connection was accepted where there is one,
    /// by pid otherwise. The pids reached.
    pub fn signal(
        &self,
        clients: &[ClientKey],
        signal: Signal,
        own: RawPid,
        spared: Option<RawPid>,
    ) -> BTreeSet<RawPid> {
        clients
            .iter()
            .filter_map(|client| match self.0.get(client) {
                Some(Peer::App(held)) => Some(held),
                Some(Peer::Satellite) | None => None,
            })
            .filter(|held| held.pid != own && Some(held.pid) != spared)
            .filter(|held| Pid::from_raw(held.pid).is_some_and(|pid| !pid.is_init()))
            .filter(|held| reach(held, signal))
            .map(|held| held.pid)
            .collect()
    }

    /// Where the toplevels of `client` come from: X11 when it is the connection
    /// xwayland-satellite carries every X11 program's windows on.
    pub fn origin_of(&self, client: ClientKey) -> WindowOrigin {
        match self.0.get(&client) {
            Some(Peer::Satellite) => WindowOrigin::X11,
            Some(Peer::App(_)) | None => WindowOrigin::Wayland,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustix::process::getpid;

    #[test]
    fn a_connection_is_known_by_its_peer_until_it_closes() {
        let (ours, _theirs) = UnixStream::pair().expect("a socket pair");
        let mut processes = Processes::default();
        processes.record(ClientKey(7), &ours, None);
        let own = getpid().as_raw_pid();
        assert_eq!(
            processes
                .of(&[ClientKey(7), ClientKey(8)])
                .collect::<Vec<_>>(),
            vec![own]
        );
        processes.forget(ClientKey(7));
        assert_eq!(processes.of(&[ClientKey(7)]).count(), 0);
    }

    /// CASB-20: the signal goes through the handle held since accept, so it reaches the process
    /// the connection had; init, this process and the spared one are left alone.
    #[test]
    fn a_connections_process_is_signalled_through_its_handle_and_others_are_spared() {
        use std::os::unix::process::ExitStatusExt;
        let mut child = std::process::Command::new("sleep")
            .arg("30")
            .spawn()
            .expect("sleep starts");
        // The socket's peer credentials name this process, not the child: record the child's
        // connection by hand, with a handle on it, as `record` would have.
        let pid = RawPid::try_from(child.id()).expect("a pid fits");
        let handle = pidfd_open(Pid::from_raw(pid).expect("a pid"), PidfdFlags::empty()).ok();
        assert!(handle.is_some(), "a pidfd can be opened for a child");
        let mut processes = Processes::default();
        processes
            .0
            .insert(ClientKey(5), Peer::App(Held { pid, handle }));
        let own = getpid().as_raw_pid();
        let reached = processes.signal(&[ClientKey(5), ClientKey(6)], Signal::KILL, own, None);
        assert_eq!(reached, BTreeSet::from([pid]));
        assert_eq!(child.wait().expect("ends").signal(), Some(9));
        // A spared pid is not signalled, nor a connection nothing is recorded for.
        let mut kept = Processes::default();
        kept.0.insert(
            ClientKey(1),
            Peer::App(Held {
                pid: own,
                handle: None,
            }),
        );
        assert!(
            kept.signal(&[ClientKey(1)], Signal::TERM, own, None)
                .is_empty()
        );
    }

    /// NEW-CASB-7: the handle is the kernel's pidfd of the connector, made at connect.
    #[test]
    fn a_connections_handle_is_the_kernels_pidfd_of_the_connector() {
        use std::os::fd::AsRawFd;
        let (ours, _theirs) = UnixStream::pair().expect("a socket pair");
        let own = getpid().as_raw_pid();
        let handle = handle_of(&ours, own).expect("a handle on this process");
        let info = std::fs::read_to_string(format!("/proc/self/fdinfo/{}", handle.as_raw_fd()))
            .expect("fdinfo");
        assert!(info.contains(&format!("Pid:\t{own}")), "{info}");
    }

    #[test]
    fn the_satellites_connection_is_never_recorded() {
        let (ours, _theirs) = UnixStream::pair().expect("a socket pair");
        let own = getpid().as_raw_pid();
        let mut processes = Processes::default();
        processes.record(ClientKey(1), &ours, Some(own));
        assert_eq!(processes.of(&[ClientKey(1)]).count(), 0, "spared");
        assert_eq!(processes.origin_of(ClientKey(1)), WindowOrigin::X11);
        processes.record(ClientKey(2), &ours, Some(own + 1));
        assert_eq!(
            processes.of(&[ClientKey(2)]).count(),
            1,
            "another pid is recorded"
        );
        assert_eq!(processes.origin_of(ClientKey(2)), WindowOrigin::Wayland);
        assert_eq!(processes.origin_of(ClientKey(3)), WindowOrigin::Wayland);
    }
}
