//! A private dbus-daemon for tests that serve or listen on a bus.

use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};

/// A dbus-daemon on the repo's `dev/private-bus.conf` with a cleared environment, killed and
/// reaped when dropped, so a failing test leaves nothing behind. It never touches the system
/// bus.
#[derive(Debug)]
pub struct PrivateBus {
    daemon: Child,
    address: String,
}

/// Why the private bus did not start.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PrivateBusError {
    /// `dbus-daemon` could not be run or its address not read.
    #[error("dbus-daemon: {0}")]
    Daemon(#[from] std::io::Error),
    /// It printed no address.
    #[error("dbus-daemon printed no address")]
    NoAddress,
}

impl PrivateBus {
    /// Starts the daemon and reads the address it listens on.
    ///
    /// ```no_run
    /// use casement_testkit::fakes::PrivateBus;
    /// let bus = PrivateBus::start().expect("bus");
    /// assert!(!bus.address().is_empty());
    /// ```
    pub fn start() -> Result<PrivateBus, PrivateBusError> {
        let config = concat!(env!("CARGO_MANIFEST_DIR"), "/../../dev/private-bus.conf");
        let daemon = Command::new("dbus-daemon")
            .env_clear()
            .args(["--config-file", config, "--nofork", "--print-address=1"])
            .stdout(Stdio::piped())
            .spawn()?;
        // The guard exists before anything can fail, so its Drop covers the read below.
        let mut bus = PrivateBus {
            daemon,
            address: String::new(),
        };
        if let Some(stdout) = bus.daemon.stdout.take() {
            BufReader::new(stdout).read_line(&mut bus.address)?;
        }
        bus.address = bus.address.trim().to_owned();
        if bus.address.is_empty() {
            return Err(PrivateBusError::NoAddress);
        }
        Ok(bus)
    }

    /// The bus address to connect to.
    pub fn address(&self) -> &str {
        &self.address
    }
}

impl Drop for PrivateBus {
    fn drop(&mut self) {
        let _ = self.daemon.kill();
        let _ = self.daemon.wait();
    }
}
