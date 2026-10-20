//! The fixture's way to a privileged connection the way a program gets one: through
//! `org.quire.Compositor1.Connect`'s decision (`casement::connect::answer`), with the caller as
//! the bus thread would have proven it (a [`Proven`] from [`fakes::callers`](crate::fakes::callers))
//! instead of a process on a bus.

use super::{ClientId, Fixture};
use casement::clients::{Judge, Probes, begin_probe};
use casement::connect;
use casement::privacy::SystemDir;
use casement_core::input::InputEvent;
use casement_core::policy::ConnectionRole;
use casement_ipc::dbus::peer::Table;
use casement_ipc::dbus::{ConnectAsk, ConnectRefusal, ConnectReply, Proven};
use casement_testclient::TestClient;
use std::os::fd::OwnedFd;
use std::os::unix::net::UnixStream;

impl Fixture {
    /// A new client asking for a connection with `role`, as `caller` proved to be. Granted, it
    /// is pumped in like [`Fixture::connect`]'s; refused, the reason is what the D-Bus caller
    /// would be told.
    /// The compositor's handle on the client is not kept, so helpers that need it
    /// (`toplevel_of`) are for [`Fixture::connect`]'s clients.
    pub fn connect_through_handout(
        &mut self,
        caller: &Proven,
        role: ConnectionRole,
    ) -> Result<ClientId, ConnectRefusal> {
        match connect::answer(&mut self.state, ConnectAsk::new(role, caller.clone())) {
            ConnectReply::Granted(fd) => {
                let end = UnixStream::from(fd);
                end.set_nonblocking(true)
                    .expect("a non-blocking client end");
                let client =
                    TestClient::over(OwnedFd::from(end)).expect("the test client connects");
                let id = ClientId(self.next_client);
                self.next_client += 1;
                self.clients.insert(id, client);
                self.pump();
                Ok(id)
            }
            ConnectReply::Refused(why) => Err(why),
            _ => Err(ConnectRefusal::Unavailable),
        }
    }

    /// `caller` asking for a remote EIS socket (`ConnectEis`'s decision,
    /// `casement::connect::answer`) for portal session `session` with the `devices` bit set.
    /// Granted, the client end of the socket; refused, the reason the D-Bus caller is told.
    pub fn connect_remote_eis(
        &mut self,
        caller: &Proven,
        session: u64,
        devices: u32,
    ) -> Result<OwnedFd, ConnectRefusal> {
        match connect::answer(
            &mut self.state,
            ConnectAsk::remote_eis(session, devices, caller.clone()),
        ) {
            ConnectReply::Granted(fd) => Ok(fd),
            ConnectReply::Refused(why) => Err(why),
            _ => Err(ConnectRefusal::Unavailable),
        }
    }

    /// The same, asked from the bus connection `owner` (its unique name): its sockets go when
    /// that connection does ([`Fixture::portal_left`]).
    pub fn connect_remote_eis_from(
        &mut self,
        caller: &Proven,
        owner: &str,
        session: u64,
        devices: u32,
    ) -> Result<OwnedFd, ConnectRefusal> {
        match connect::answer(
            &mut self.state,
            ConnectAsk::remote_eis(session, devices, caller.clone()).from_owner(owner),
        ) {
            ConnectReply::Granted(fd) => Ok(fd),
            ConnectReply::Refused(why) => Err(why),
            _ => Err(ConnectRefusal::Unavailable),
        }
    }

    /// The portal's bus connection `owner` left the bus: the decision casement's loop makes when
    /// the bus thread says so (`Compositor1`'s name watch).
    pub fn portal_left(&mut self, portal: &Proven, owner: &str) {
        let _ = connect::answer(
            &mut self.state,
            ConnectAsk::owner_gone(owner, portal.clone()),
        );
    }

    /// A program connects on the public socket the way `casement::run` takes it in: the look at
    /// its facts runs on a thread, and the connection is inserted when the look is done (or its
    /// deadline passes). Returns the client end and how many connections the session holds more
    /// than before, once the look has had its time.
    pub fn public_arrival(&mut self) -> (UnixStream, usize) {
        let (server, client) = UnixStream::pair().expect("a socket pair");
        if matches!(self.state.probes, Probes::Off) {
            let handle = self.state.event_loop.clone();
            self.state.probes = Probes::start(Judge::new(Table::empty(), SystemDir::Etc), &handle)
                .expect("the looks join the loop");
        }
        let before = self.state.roster.len();
        begin_probe(&mut self.state, server);
        for _ in 0..400 {
            self.pump();
            if self.state.roster.len() > before {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        (client, self.state.roster.len() - before)
    }

    /// `caller` asking to end portal session `session`'s remote sockets (`EndEis`'s decision,
    /// `casement::connect::answer`). Granted, nothing; refused, the reason the D-Bus caller is
    /// told.
    pub fn end_remote_eis(&mut self, caller: &Proven, session: u64) -> Result<(), ConnectRefusal> {
        match connect::answer(
            &mut self.state,
            ConnectAsk::end_eis(session, caller.clone()),
        ) {
            ConnectReply::Ended => Ok(()),
            ConnectReply::Refused(why) => Err(why),
            _ => Err(ConnectRefusal::Unavailable),
        }
    }

    /// `caller` asking for a lease on the window whose foreign-toplevel identifier is `window`
    /// (`LeaseWindow`'s decision, `casement::connect::answer`) with the `devices` bit set.
    /// Granted, the lease's number and the client end of its input socket; refused, the reason
    /// the D-Bus caller is told.
    pub fn lease_window(
        &mut self,
        caller: &Proven,
        window: &str,
        devices: u32,
    ) -> Result<(u64, OwnedFd), ConnectRefusal> {
        match connect::answer(
            &mut self.state,
            ConnectAsk::lease_window(window, devices, caller.clone()),
        ) {
            ConnectReply::Leased { lease, socket } => Ok((lease, socket)),
            ConnectReply::Refused(why) => Err(why),
            _ => Err(ConnectRefusal::Unavailable),
        }
    }

    /// `caller` asking to end `lease` (`EndLease`'s decision). Granted, nothing; refused, the
    /// reason the D-Bus caller is told.
    pub fn end_lease(&mut self, caller: &Proven, lease: u64) -> Result<(), ConnectRefusal> {
        match connect::answer(
            &mut self.state,
            ConnectAsk::end_lease(lease, caller.clone()),
        ) {
            ConnectReply::Ended => Ok(()),
            ConnectReply::Refused(why) => Err(why),
            _ => Err(ConnectRefusal::Unavailable),
        }
    }

    /// An input event from a remote session, by the path a remote socket's input takes into
    /// the model, then a pump.
    pub fn deliver_remote(&mut self, event: InputEvent) {
        connect::deliver_remote(&mut self.state, event);
        self.pump();
    }
}
