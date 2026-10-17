//! Clients that arrive with a standing: who the compositor takes their program to be, which
//! decides what they may use without asking (`casement_core::privilege`).
//!
//! The protocol tests connect a public client as a desktop service the table trusts, so the
//! powers they exercise (capture, the clipboard, virtual keys) are not asked about; a test of
//! the consent flow itself connects as a program that is asked ([`Fixture::connect_via_ask`]) or
//! as any standing it names ([`Fixture::connect_standing`]).

use super::{ClientId, Fixture};
use casement::clients::{self, Standing};
use casement_core::policy::ConnectionRole;
use casement_core::privilege::{ClientStanding, ProgramPath, ServiceRole};
use casement_wayland::consent::{ImageWatch, Unwatched};
use casement_wayland::security_context::Maker;
use porter_core::PeerIdentity;
use smithay::reexports::wayland_server::Client;
use std::os::unix::net::UnixStream;
use std::sync::Arc;

/// The standing of the desktop's own shell: trusted with every power, never asked.
pub fn trusted_standing() -> ClientStanding {
    ClientStanding::Service {
        role: ServiceRole::parse("shell").expect("a role in the grammar"),
        exe: None,
    }
}

/// The standing of a program the system owns, at `path`: asked, and remembered by its path.
pub fn program_standing(path: &str) -> ClientStanding {
    ClientStanding::Program {
        exe: ProgramPath::parse(path).expect("an absolute path"),
    }
}

/// The program [`Fixture::connect_via_ask`] connects as.
pub const ASKED_PROGRAM: &str = "/usr/bin/grim";

impl Fixture {
    /// A new public client whose program is taken to be `standing`.
    pub fn connect_standing(&mut self, standing: ClientStanding) -> ClientId {
        self.join(ConnectionRole::Public, Some(standing), None)
    }

    /// A new public client taken to be `standing`, whose image `watch` looks at when it first
    /// uses a power (a program that executes another image after it connected).
    pub fn connect_watched(
        &mut self,
        standing: ClientStanding,
        watch: Arc<dyn ImageWatch>,
    ) -> ClientId {
        self.join(ConnectionRole::Public, Some(standing), Some(watch))
    }

    /// A new public client that is [`ASKED_PROGRAM`]: the first use of each power asks the
    /// person, through the shell.
    pub fn connect_via_ask(&mut self) -> ClientId {
        self.connect_standing(program_standing(ASKED_PROGRAM))
    }

    /// How many clients have objects on hold in a protocol smithay serves whole (the clipboard
    /// managers, the input method, the foreign toplevel list), waiting for the person's answer.
    pub fn held_connections(&self) -> usize {
        self.state().wl.clients.ledger().parked_count()
    }

    /// Inserts the compositor's end of a new connection through the composition root's own door
    /// (`casement::clients::insert_client`). With a standing, the way the root does it for a
    /// public client: the client data carries the standing and the globals the person's book
    /// hides from it, and the model hears of it right after the connection.
    pub(super) fn insert(
        &mut self,
        server: UnixStream,
        role: ConnectionRole,
        standing: Option<ClientStanding>,
        watch: Option<Arc<dyn ImageWatch>>,
    ) -> Client {
        let arrival = match standing {
            Some(standing) => Standing::Judged {
                standing,
                watch: watch.unwrap_or_else(|| Arc::new(Unwatched)),
                maker: Maker::Other,
            },
            None => Standing::Unjudged,
        };
        clients::insert_client(
            &mut self.state,
            server,
            role,
            PeerIdentity::Unproven,
            arrival,
        )
        .expect("the compositor takes the client")
    }
}
