//! Who the program on a Wayland socket is, read when it connects: the process is held by a
//! descriptor, its facts are read from `/proc`, and they are judged against the desktop's
//! caller table (`/etc/quire/desktop-callers.toml`, root-owned or not believed).

use super::Standing;
use super::image::PinnedImage;
use super::standing::{client_standing, peer_of};
use casement_core::privilege::ClientStanding;
use casement_ipc::dbus::peer::{Pinned, Prober, Table, judge};
use casement_ipc::dbus::{RealProc, TableFiles};
use casement_wayland::consent::Unwatched;
use porter_core::PeerIdentity;
use std::os::unix::net::UnixStream;
use std::sync::Arc;

/// What is known of a connection's program the moment it arrives.
#[derive(Debug, Clone)]
pub struct Arrival {
    /// The app its windows belong to.
    pub peer: PeerIdentity,
    /// How well its program is known.
    pub standing: Standing,
}

impl Arrival {
    /// A program nothing could be learned of: asked about every power, every time.
    pub fn unknown() -> Arrival {
        Arrival {
            peer: PeerIdentity::Unproven,
            standing: Standing::Judged {
                standing: ClientStanding::unproven(),
                watch: Arc::new(Unwatched),
            },
        }
    }
}

/// The desktop's own list of its programs, `/etc/quire/desktop-callers.toml` and its drop-ins. A
/// table that cannot be loaded (missing, or not owned by the administrator alone) is an empty
/// one, where no program is one of the desktop's services: it is logged once and the compositor
/// runs, with nobody privileged and the shell unable to connect.
pub fn desktop_table() -> Table {
    match TableFiles::desktop().load(&RealProc::system()) {
        Ok(table) => table,
        Err(error) => {
            tracing::error!(
                %error,
                "the desktop's list of its own programs could not be used; no program is taken for one of them, and the shell cannot connect"
            );
            Table::empty()
        }
    }
}

/// Judges the programs that connect against the desktop's caller table.
#[derive(Debug)]
pub struct Judge {
    prober: Prober<RealProc>,
    table: Table,
}

impl Judge {
    /// A judge over `table`, reading the system's `/proc`.
    pub fn new(table: Table) -> Judge {
        Judge {
            prober: Prober::system(),
            table,
        }
    }

    /// A judge over the desktop's own table ([`desktop_table`]).
    pub fn desktop() -> Judge {
        Judge::new(desktop_table())
    }

    /// Who the program on the other end of `stream` is.
    pub fn arrival(&self, stream: &UnixStream) -> Arrival {
        let Ok(pinned) = Pinned::of_stream(stream) else {
            return Arrival::unknown();
        };
        let Ok(facts) = self.prober.probe(&pinned, None) else {
            return Arrival::unknown();
        };
        let found = judge(&facts, &self.table);
        let standing = client_standing(&found, &facts.exe);
        Arrival {
            peer: peer_of(&found),
            standing: Standing::Judged {
                standing,
                watch: Arc::new(PinnedImage::new(pinned, self.prober.clone(), facts.exe)),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_program_is_held_looked_at_and_judged_when_it_connects() {
        let (ours, _theirs) = UnixStream::pair().expect("a socket pair");
        let arrival = Judge::new(Table::empty()).arrival(&ours);
        let Standing::Judged { standing, watch } = arrival.standing else {
            panic!("a public client is always judged");
        };
        // This test binary is no desktop service (the table is empty): a program the system
        // owns, or one the person can write (the build folder), or one that could not be read.
        assert!(
            matches!(
                standing,
                ClientStanding::Program { .. } | ClientStanding::Unverified { .. }
            ),
            "{standing:?}"
        );
        assert!(matches!(arrival.peer, PeerIdentity::Unproven));
        assert!(watch.moved().is_none(), "it still runs the image it had");
    }
}
