//! Telling systemd how the service is (`sd_notify`): ready once the Wayland socket listens and
//! the first outputs are announced, stopping on the way out, and a watchdog ping while the loop
//! turns.
//!
//! [`Notify`] is the seam. [`SocketNotify`] writes one datagram per [`Notice`] to the socket
//! systemd names in `$NOTIFY_SOCKET` (the binary reads it and passes an [`Endpoint`]; the
//! library reads no environment); with no socket nothing is sent. `casement-testkit` exports
//! `FakeNotify`. The watchdog period is read once by the binary ([`Watchdog::from_env`]).
//!
//! ```
//! use casement::endpoint::Endpoint;
//! use casement::notify::{Notice, Notify, SocketNotify};
//!
//! let mut notify = SocketNotify::new(Endpoint::Unset);
//! notify.send(&Notice::Stopping).expect("nothing to send to is not an error");
//! assert_eq!(Notice::Watchdog.datagram(), "WATCHDOG=1");
//! ```

use crate::endpoint::Endpoint;
use std::os::linux::net::SocketAddrExt;
use std::os::unix::net::{SocketAddr, UnixDatagram};
use std::time::Duration;

/// What casement tells the service manager.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Notice {
    /// Startup is done; `status` says what it is now doing, in plain words.
    Ready {
        /// The status line `systemctl status` shows.
        status: String,
    },
    /// The compositor is on its way out.
    Stopping,
    /// The loop is alive (the watchdog).
    Watchdog,
}

impl Notice {
    /// The datagram's text (newline-separated `KEY=value` lines).
    pub fn datagram(&self) -> String {
        match self {
            Notice::Ready { status } => {
                format!("READY=1\nSTATUS={}", status.replace(['\n', '\r'], " "))
            }
            Notice::Stopping => "STOPPING=1\nSTATUS=Stopping".to_owned(),
            Notice::Watchdog => "WATCHDOG=1".to_owned(),
        }
    }
}

/// Why a notice could not be sent.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum NotifyError {
    /// The socket could not be reached or written.
    #[error("the notification socket {socket}: {source}")]
    Socket {
        /// The socket as systemd named it.
        socket: String,
        /// Why.
        #[source]
        source: std::io::Error,
    },
}

/// Where notices go.
pub trait Notify: std::fmt::Debug {
    /// Sends `notice`.
    fn send(&mut self, notice: &Notice) -> Result<(), NotifyError>;
}

/// The real implementation: a datagram to the named socket, std's `UnixDatagram`.
#[derive(Debug, Clone)]
pub struct SocketNotify {
    socket: Endpoint,
}

impl SocketNotify {
    /// Notices to `socket`.
    pub fn new(socket: Endpoint) -> SocketNotify {
        SocketNotify { socket }
    }
}

impl Notify for SocketNotify {
    fn send(&mut self, notice: &Notice) -> Result<(), NotifyError> {
        let Endpoint::At(name) = &self.socket else {
            return Ok(());
        };
        let wrap = |source| NotifyError::Socket {
            socket: name.clone(),
            source,
        };
        let socket = UnixDatagram::unbound().map_err(wrap)?;
        let bytes = notice.datagram();
        match name.strip_prefix('@') {
            Some(abstract_name) => {
                let address =
                    SocketAddr::from_abstract_name(abstract_name.as_bytes()).map_err(wrap)?;
                socket.send_to_addr(bytes.as_bytes(), &address)
            }
            None => socket.send_to(bytes.as_bytes(), name),
        }
        .map(drop)
        .map_err(wrap)
    }
}

/// How often systemd wants a ping: off, or every `Duration` (half of `WatchdogSec`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Watchdog {
    /// No watchdog is set for this process.
    #[default]
    Off,
    /// Ping this often.
    Every(Duration),
}

impl Watchdog {
    /// The watchdog `$WATCHDOG_USEC` and `$WATCHDOG_PID` describe for the process `own_pid`:
    /// off when the period is missing, zero or not a number, or when the variable names another
    /// process.
    ///
    /// ```
    /// use casement::notify::Watchdog;
    /// use std::time::Duration;
    ///
    /// assert_eq!(Watchdog::from_env(Some("20000000"), None, 7), Watchdog::Every(Duration::from_secs(10)));
    /// assert_eq!(Watchdog::from_env(Some("20000000"), Some("8"), 7), Watchdog::Off);
    /// assert_eq!(Watchdog::from_env(None, None, 7), Watchdog::Off);
    /// ```
    pub fn from_env(usec: Option<&str>, pid: Option<&str>, own_pid: u32) -> Watchdog {
        let addressed_here = pid.is_none_or(|text| text.trim().parse::<u32>() == Ok(own_pid));
        let period = usec.and_then(|text| text.trim().parse::<u64>().ok());
        match (period, addressed_here) {
            (Some(micros), true) if micros > 1 => {
                Watchdog::Every(Duration::from_micros(micros / 2))
            }
            _ => Watchdog::Off,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn datagrams_are_the_lines_systemd_reads() {
        let ready = Notice::Ready {
            status: "listening on wayland-1\nsecond".to_owned(),
        };
        assert_eq!(
            ready.datagram(),
            "READY=1\nSTATUS=listening on wayland-1 second"
        );
        assert_eq!(Notice::Stopping.datagram(), "STOPPING=1\nSTATUS=Stopping");
        assert_eq!(Notice::Watchdog.datagram(), "WATCHDOG=1");
    }

    #[test]
    fn the_socket_gets_one_datagram_per_notice() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("notify");
        let listener = UnixDatagram::bind(&path).expect("bind");
        let mut notify = SocketNotify::new(Endpoint::At(path.display().to_string()));
        notify.send(&Notice::Watchdog).expect("sent");
        notify.send(&Notice::Stopping).expect("sent");
        let mut buf = [0u8; 128];
        let n = listener.recv(&mut buf).expect("first");
        assert_eq!(&buf[..n], b"WATCHDOG=1");
        let n = listener.recv(&mut buf).expect("second");
        assert_eq!(&buf[..n], b"STOPPING=1\nSTATUS=Stopping");
    }

    #[test]
    fn an_abstract_socket_is_named_with_an_at_sign() {
        let name = format!("casement-notify-test-{}", std::process::id());
        let address = SocketAddr::from_abstract_name(name.as_bytes()).expect("address");
        let listener = UnixDatagram::bind_addr(&address).expect("bind");
        let mut notify = SocketNotify::new(Endpoint::At(format!("@{name}")));
        notify.send(&Notice::Watchdog).expect("sent");
        let mut buf = [0u8; 16];
        let n = listener.recv(&mut buf).expect("received");
        assert_eq!(&buf[..n], b"WATCHDOG=1");
    }

    #[test]
    fn an_unset_socket_sends_nothing_and_a_missing_one_is_an_error() {
        let mut none = SocketNotify::new(Endpoint::Unset);
        none.send(&Notice::Stopping).expect("nothing to do");
        let mut gone = SocketNotify::new(Endpoint::At("/nonexistent/notify".to_owned()));
        assert!(gone.send(&Notice::Stopping).is_err());
    }

    #[test]
    fn the_watchdog_pings_at_half_the_period_for_this_process_only() {
        let cases = [
            (
                Some("20000000"),
                None,
                Watchdog::Every(Duration::from_secs(10)),
            ),
            (
                Some("20000000"),
                Some("7"),
                Watchdog::Every(Duration::from_secs(10)),
            ),
            (Some("20000000"), Some("8"), Watchdog::Off),
            (Some("0"), None, Watchdog::Off),
            (Some("soon"), None, Watchdog::Off),
            (None, Some("7"), Watchdog::Off),
        ];
        for (usec, pid, want) in cases {
            assert_eq!(Watchdog::from_env(usec, pid, 7), want, "{usec:?} {pid:?}");
        }
    }
}
