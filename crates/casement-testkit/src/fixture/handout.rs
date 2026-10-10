//! The fixture's way to a privileged connection the way a program gets one: through
//! `org.quire.Compositor1.Connect`'s decision (`casement::connect::answer`), with the caller's
//! control group read from a fake process table instead of `/proc`.

use super::{ClientId, Fixture};
use casement::connect;
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
}
