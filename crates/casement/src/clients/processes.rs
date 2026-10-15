//! The process behind each public connection, read from the socket's peer credentials when the
//! client connects. Force quit signals these (`exec/quit.rs`); the display's own client list is
//! never walked for it.

use casement_core::ClientKey;
use rustix::process::RawPid;
use std::collections::BTreeMap;
use std::os::unix::net::UnixStream;

/// The process of each public connection still open.
#[derive(Debug, Default)]
pub struct Processes(BTreeMap<ClientKey, RawPid>);

impl Processes {
    /// Records the process on the other end of `stream` as `client`'s, when the kernel names one.
    pub fn record(&mut self, client: ClientKey, stream: &UnixStream) {
        if let Ok(peer) = rustix::net::sockopt::socket_peercred(stream) {
            self.0.insert(client, peer.pid.as_raw_pid());
        }
    }

    /// Drops `client`'s process: its connection closed.
    pub fn forget(&mut self, client: ClientKey) {
        self.0.remove(&client);
    }

    /// The recorded processes of `clients`.
    pub fn of(&self, clients: &[ClientKey]) -> impl Iterator<Item = RawPid> {
        clients
            .iter()
            .filter_map(|client| self.0.get(client).copied())
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
        processes.record(ClientKey(7), &ours);
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
}
