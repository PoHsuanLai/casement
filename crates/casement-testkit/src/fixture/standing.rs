//! Clients that arrive with a standing: who the compositor takes their program to be, which
//! decides what they may use without asking (`casement_core::privilege`).
//!
//! The protocol tests connect a public client as a desktop service the table trusts, so the
//! powers they exercise (capture, the clipboard, virtual keys) are not asked about; a test of
//! the consent flow itself connects as a program that is asked ([`Fixture::connect_via_ask`]) or
//! as any standing it names ([`Fixture::connect_standing`]).

use super::{ClientId, Fixture};
use casement::{clients, core_loop};
use casement_core::Event;
use casement_core::policy::ConnectionRole;
use casement_core::privilege::{ClientStanding, ProgramPath, ServiceRole};
use casement_wayland::ClientState;
use casement_wayland::consent::ImageWatch;
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

    /// Inserts the compositor's end of a new connection. With a standing, the way the
    /// composition root does it for a public client: the client data carries the standing and
    /// the globals the person's book hides from it, and the model hears of it right after the
    /// connection.
    pub(super) fn insert(
        &mut self,
        server: UnixStream,
        role: ConnectionRole,
        standing: Option<ClientStanding>,
        watch: Option<Arc<dyn ImageWatch>>,
    ) -> Client {
        let Some(standing) = standing else {
            return clients::insert_client(&mut self.state, server, role, PeerIdentity::Unproven)
                .expect("the compositor takes the client");
        };
        let state = &mut self.state;
        let (mint, key) = state.wl.clients.clone().next();
        state.wl.clients = mint;
        let mut data = ClientState::new(key, role.clone(), PeerIdentity::Unproven)
            .with_standing_under(state.core.model.privilege(), standing.clone())
            .with_gone(state.wl.clients.gone_queue());
        if let Some(watch) = watch {
            data = data.with_image_watch(watch);
        }
        let held = state
            .display
            .insert_client(server, Arc::new(data))
            .expect("the display takes the client");
        core_loop::feed(
            state,
            vec![
                Event::ClientConnected {
                    client: key,
                    peer: PeerIdentity::Unproven,
                    role,
                },
                Event::ClientStanding {
                    client: key,
                    standing,
                },
            ],
        );
        held
    }
}
