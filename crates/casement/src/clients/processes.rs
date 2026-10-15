//! The process behind each public connection, read from the socket's peer credentials when the
//! client connects. Force quit signals these (`exec/quit.rs`); the display's own client list is
//! never walked for it. xwayland-satellite connects through the public socket like any app, but
//! it carries every X11 program's windows and is not an app's process: it is never recorded as
//! one. Its connection is remembered as an X11 carrier, so force quit knows to ask the X server
//! for the processes behind the windows (`xwayland::owners`).

use casement_core::ClientKey;
use rustix::process::RawPid;
use std::collections::BTreeMap;
use std::os::unix::net::UnixStream;

/// What is on the other end of a public connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Peer {
    /// An app's own process.
    App(RawPid),
    /// xwayland-satellite, whose one connection carries every X11 program's windows.
    Satellite,
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
                false => Peer::App(pid),
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
                Some(Peer::App(pid)) => Some(*pid),
                Some(Peer::Satellite) | None => None,
            })
    }

    /// Whether any of `clients` is the connection xwayland-satellite carries X11 windows on.
    pub fn carries_x11(&self, clients: &[ClientKey]) -> bool {
        clients
            .iter()
            .any(|client| self.0.get(client) == Some(&Peer::Satellite))
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

    #[test]
    fn the_satellites_connection_is_never_recorded() {
        let (ours, _theirs) = UnixStream::pair().expect("a socket pair");
        let own = getpid().as_raw_pid();
        let mut processes = Processes::default();
        processes.record(ClientKey(1), &ours, Some(own));
        assert_eq!(processes.of(&[ClientKey(1)]).count(), 0, "spared");
        assert!(processes.carries_x11(&[ClientKey(1)]), "an X11 carrier");
        processes.record(ClientKey(2), &ours, Some(own + 1));
        assert_eq!(
            processes.of(&[ClientKey(2)]).count(),
            1,
            "another pid is recorded"
        );
        assert!(!processes.carries_x11(&[ClientKey(2), ClientKey(3)]));
    }
}
