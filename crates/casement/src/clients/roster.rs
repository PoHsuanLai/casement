//! Who is connected and how many connections one program may hold (NEW-CAS-1, CASC-03). Every
//! connection costs the compositor a socket and a process handle, and the loop is one thread:
//! a program that opens connections without end would use up the descriptors, so the number of
//! live connections is capped for the whole session and for each holder (an app when its name
//! is proven, otherwise the process). A connection past a cap is refused at accept, closed
//! before anything is read from the program behind it. The last few connections of the session
//! are kept for apps and services whose name is proven ([`RESERVED_FOR_PROVEN`]).

use casement_core::ClientKey;
use porter_core::{AppId, PeerIdentity};
use rustix::process::RawPid;
use std::collections::BTreeMap;
use std::os::unix::net::UnixStream;

/// Live connections the session takes in all: two descriptors each (the socket and the held
/// process), well under the soft limit `limits::raise_open_files` asks for.
pub const MAX_CONNECTIONS: usize = 384;

/// The last connections of the session's cap, kept for apps and services whose name is proven
/// ("New 2"): a program nothing proves a name for is refused once the session holds
/// `MAX_CONNECTIONS - RESERVED_FOR_PROVEN`, so a few hostile processes cannot fill the session
/// and leave no room for the next program the person starts. The cap is still checked before
/// anything is read from the program (it cannot yet be told apart then); the reserve is applied
/// when its look has finished.
pub const RESERVED_FOR_PROVEN: usize = 32;

/// Live connections one app (or one process, when no name is proven) may hold: browsers and
/// toolkits open a handful, a program that opens dozens more is looping.
pub const MAX_PER_HOLDER: usize = 48;

/// Who a connection counts against.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Holder {
    /// An app whose name is proven: all its processes share the cap.
    App(AppId),
    /// A process nothing proves a name for.
    Process(RawPid),
    /// A connection whose process the kernel could not name.
    Unknown,
}

impl Holder {
    /// The holder of a connection whose proven identity is `peer` and whose process is `process`.
    pub fn of(peer: &PeerIdentity, process: Option<RawPid>) -> Holder {
        match (peer, process) {
            (PeerIdentity::Proven(app), _) => Holder::App(app.clone()),
            // Unproven, or a kind of proof this build does not know: counted per process.
            (_, Some(pid)) => Holder::Process(pid),
            (_, None) => Holder::Unknown,
        }
    }

    /// The holder of `stream`, read from its peer credentials.
    pub fn of_stream(peer: &PeerIdentity, stream: &UnixStream) -> Holder {
        let process = rustix::net::sockopt::socket_peercred(stream)
            .ok()
            .map(|cred| cred.pid.as_raw_pid());
        Holder::of(peer, process)
    }
}

/// Whether one more connection fits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Room {
    /// It fits.
    Free,
    /// The session holds the most connections it takes.
    SessionFull,
    /// This holder already has the most it may.
    HolderFull,
}

/// The live connections and who holds each.
#[derive(Debug, Default)]
pub struct Roster {
    held: BTreeMap<ClientKey, Holder>,
    per_holder: BTreeMap<Holder, usize>,
    refused: u64,
}

impl Roster {
    /// Whether the session takes one more connection from anyone.
    pub fn room(&self) -> Room {
        match self.held.len() < MAX_CONNECTIONS {
            true => Room::Free,
            false => Room::SessionFull,
        }
    }

    /// Whether the session takes one more connection from `holder`: the connections a program
    /// nothing proves a name for may hold stop short of the reserve ([`RESERVED_FOR_PROVEN`]).
    pub fn room_for(&self, holder: &Holder) -> Room {
        let ceiling = match holder {
            Holder::App(_) => MAX_CONNECTIONS,
            Holder::Process(_) | Holder::Unknown => MAX_CONNECTIONS - RESERVED_FOR_PROVEN,
        };
        match (self.room(), self.held.len() < ceiling) {
            (Room::Free, true) => {
                match self.per_holder.get(holder).copied().unwrap_or(0) < MAX_PER_HOLDER {
                    true => Room::Free,
                    false => Room::HolderFull,
                }
            }
            (Room::Free, false) => Room::SessionFull,
            (full, _) => full,
        }
    }

    /// `client` connected, held by `holder`.
    pub fn join(&mut self, client: ClientKey, holder: Holder) {
        *self.per_holder.entry(holder.clone()).or_insert(0) += 1;
        if let Some(replaced) = self.held.insert(client, holder) {
            self.release(&replaced);
        }
    }

    /// `client` disconnected.
    pub fn leave(&mut self, client: ClientKey) {
        if let Some(holder) = self.held.remove(&client) {
            self.release(&holder);
        }
    }

    /// One fewer connection for `holder`; a holder with none is forgotten.
    fn release(&mut self, holder: &Holder) {
        if let Some(count) = self.per_holder.get_mut(holder) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                self.per_holder.remove(holder);
            }
        }
    }

    /// How many connections are live.
    pub fn len(&self) -> usize {
        self.held.len()
    }

    /// Whether no connection is live.
    pub fn is_empty(&self) -> bool {
        self.held.is_empty()
    }

    /// A connection was refused; the number refused so far. A flood is logged at each doubling,
    /// not once per connection.
    pub fn refused(&mut self) -> u64 {
        self.refused = self.refused.saturating_add(1);
        self.refused
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(n: u64) -> ClientKey {
        ClientKey(n)
    }

    #[test]
    fn a_holder_gets_its_cap_and_no_more_and_a_leaver_makes_room() {
        let mut roster = Roster::default();
        let me = Holder::Process(100);
        for n in 0..MAX_PER_HOLDER as u64 {
            assert_eq!(roster.room_for(&me), Room::Free, "connection {n}");
            roster.join(key(n), me.clone());
        }
        assert_eq!(roster.room_for(&me), Room::HolderFull);
        assert_eq!(
            roster.room_for(&Holder::Process(101)),
            Room::Free,
            "others are unaffected"
        );
        roster.leave(key(0));
        assert_eq!(roster.room_for(&me), Room::Free);
    }

    #[test]
    fn the_session_takes_no_more_than_the_global_cap_whoever_asks() {
        let mut roster = Roster::default();
        for n in 0..MAX_CONNECTIONS as u64 {
            // One holder per connection: the per-holder cap is never the one that trips.
            let pid = RawPid::try_from(n + 1).unwrap_or(1);
            assert_eq!(roster.room(), Room::Free);
            roster.join(key(n), Holder::Process(pid));
        }
        assert_eq!(roster.room(), Room::SessionFull);
        assert_eq!(roster.room_for(&Holder::Unknown), Room::SessionFull);
        roster.leave(key(3));
        assert_eq!(roster.room(), Room::Free);
        assert_eq!(roster.len(), MAX_CONNECTIONS - 1);
    }

    /// "New 2": past `MAX_CONNECTIONS - RESERVED_FOR_PROVEN` only a proven app or service is taken
    /// in, up to the session's cap; a process nothing proves a name for is refused.
    #[test]
    fn the_last_connections_are_kept_for_apps_whose_name_is_proven() {
        let app = |name: &str| {
            Holder::App(AppId {
                name: porter_core::AppName::parse(name).expect("a reverse-DNS name"),
                isolation: porter_core::Isolation::Unsandboxed,
            })
        };
        let mut roster = Roster::default();
        let open = MAX_CONNECTIONS - RESERVED_FOR_PROVEN;
        for n in 0..open as u64 {
            let pid = RawPid::try_from(n + 1).unwrap_or(1);
            roster.join(key(n), Holder::Process(pid));
        }
        let cases = [
            (
                "a process nothing names",
                Holder::Process(9_999),
                Room::SessionFull,
            ),
            (
                "a connection nothing could name",
                Holder::Unknown,
                Room::SessionFull,
            ),
            ("a proven app", app("org.example.Notes"), Room::Free),
        ];
        for (case, holder, want) in cases {
            assert_eq!(roster.room_for(&holder), want, "{case}");
        }
        // The reserve fills with proven apps, then the session is full for everyone.
        for n in 0..RESERVED_FOR_PROVEN as u64 {
            let holder = app(&format!("org.example.App{n}"));
            assert_eq!(
                roster.room_for(&holder),
                Room::Free,
                "proven connection {n}"
            );
            roster.join(key(open as u64 + n), holder);
        }
        assert_eq!(roster.room_for(&app("org.example.Late")), Room::SessionFull);
        assert_eq!(roster.len(), MAX_CONNECTIONS);
    }

    #[test]
    fn a_proven_app_is_one_holder_across_its_processes_and_an_unproven_one_is_its_process() {
        let app = PeerIdentity::Proven(AppId {
            name: porter_core::AppName::parse("org.example.App").expect("a reverse-DNS name"),
            isolation: porter_core::Isolation::Flatpak,
        });
        assert_eq!(Holder::of(&app, Some(1)), Holder::of(&app, Some(2)));
        assert_ne!(
            Holder::of(&PeerIdentity::Unproven, Some(1)),
            Holder::of(&PeerIdentity::Unproven, Some(2))
        );
        assert_eq!(Holder::of(&PeerIdentity::Unproven, None), Holder::Unknown);
    }

    #[test]
    fn a_flood_is_counted_and_a_rejoin_does_not_double_count() {
        let mut roster = Roster::default();
        roster.join(key(1), Holder::Unknown);
        roster.join(key(1), Holder::Unknown);
        assert_eq!(roster.len(), 1);
        roster.leave(key(1));
        assert!(roster.is_empty());
        assert_eq!(roster.refused(), 1);
        assert_eq!(roster.refused(), 2);
    }
}
