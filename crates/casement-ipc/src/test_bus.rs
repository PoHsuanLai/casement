//! A private dbus-daemon for the tests of this crate: the repo's `dev/private-bus.conf`, no
//! environment, no service directories, killed and reaped when dropped. Nothing here reaches the
//! person's real session bus.

use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use zbus::Connection;

/// A running private bus.
pub(crate) struct PrivateBus {
    daemon: Child,
    /// Where clients connect.
    pub(crate) address: String,
}

impl PrivateBus {
    pub(crate) fn start() -> PrivateBus {
        let config = concat!(env!("CARGO_MANIFEST_DIR"), "/../../dev/private-bus.conf");
        let daemon = Command::new("dbus-daemon")
            .env_clear()
            .args(["--config-file", config, "--nofork", "--print-address=1"])
            .stdout(Stdio::piped())
            .spawn()
            .expect("dbus-daemon");
        let mut bus = PrivateBus {
            daemon,
            address: String::new(),
        };
        let stdout = bus.daemon.stdout.take().expect("stdout");
        BufReader::new(stdout)
            .read_line(&mut bus.address)
            .expect("address");
        bus.address = bus.address.trim().to_owned();
        assert!(!bus.address.is_empty(), "dbus-daemon printed no address");
        bus
    }

    /// A new client connection on this bus.
    pub(crate) async fn client(&self) -> Connection {
        zbus::connection::Builder::address(self.address.as_str())
            .expect("address")
            .build()
            .await
            .expect("client")
    }
}

impl Drop for PrivateBus {
    fn drop(&mut self) {
        let _ = self.daemon.kill();
        let _ = self.daemon.wait();
    }
}

/// The unique bus name of `connection`.
pub(crate) fn name_of(connection: &Connection) -> String {
    connection
        .unique_name()
        .map(|name| name.as_str().to_owned())
        .expect("a unique name")
}
