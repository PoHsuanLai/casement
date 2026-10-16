//! The fixture's way to a privileged connection the way a program gets one: through
//! `org.quire.Compositor1.Connect`'s decision (`casement::connect::answer`), with the caller's
//! control group read from a fake process table instead of `/proc`.

use super::{ClientId, Fixture};
use casement::connect;
use casement_core::input::InputEvent;
use casement_core::policy::ConnectionRole;
use casement_ipc::dbus::{ConnectAsk, ConnectRefusal, ConnectReply, PeerCgroups};
use casement_testclient::TestClient;
use std::os::fd::OwnedFd;
use std::os::unix::net::UnixStream;

impl Fixture {
    /// A new client asking for a connection with `role` as process `pid`, whose control group
    /// `cgroups` says (a [`FakeCgroups`](crate::fakes::FakeCgroups)). Granted, it is pumped in
    /// like [`Fixture::connect`]'s; refused, the reason is what the D-Bus caller would be told.
    /// The compositor's handle on the client is not kept, so helpers that need it
    /// (`toplevel_of`) are for [`Fixture::connect`]'s clients.
    pub fn connect_through_handout(
        &mut self,
        cgroups: &dyn PeerCgroups,
        pid: u32,
        role: ConnectionRole,
    ) -> Result<ClientId, ConnectRefusal> {
        let text = cgroups
            .cgroup_of(pid)
            .map_err(|_| ConnectRefusal::UnknownUnit)?;
        match connect::answer(&mut self.state, ConnectAsk::new(role, text)) {
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

    /// The portal asking for a remote EIS socket as process `pid` (`ConnectEis`'s decision,
    /// `casement::connect::answer`) for portal session `session` with the `devices` bit set.
    /// Granted, the client end of the socket; refused, the reason the D-Bus caller is told.
    pub fn connect_remote_eis(
        &mut self,
        cgroups: &dyn PeerCgroups,
        pid: u32,
        session: u64,
        devices: u32,
    ) -> Result<OwnedFd, ConnectRefusal> {
        let text = cgroups
            .cgroup_of(pid)
            .map_err(|_| ConnectRefusal::UnknownUnit)?;
        match connect::answer(
            &mut self.state,
            ConnectAsk::remote_eis(session, devices, text),
        ) {
            ConnectReply::Granted(fd) => Ok(fd),
            ConnectReply::Refused(why) => Err(why),
            _ => Err(ConnectRefusal::Unavailable),
        }
    }

    /// The portal asking to end portal session `session`'s remote sockets as process `pid`
    /// (`EndEis`'s decision, `casement::connect::answer`). Granted, nothing; refused, the reason
    /// the D-Bus caller is told.
    pub fn end_remote_eis(
        &mut self,
        cgroups: &dyn PeerCgroups,
        pid: u32,
        session: u64,
    ) -> Result<(), ConnectRefusal> {
        let text = cgroups
            .cgroup_of(pid)
            .map_err(|_| ConnectRefusal::UnknownUnit)?;
        match connect::answer(&mut self.state, ConnectAsk::end_eis(session, text)) {
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
