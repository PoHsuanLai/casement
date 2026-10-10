//! `ZbusSpaceRegistry` over a private dbus-daemon with a stand-in `org.quire.Spaces1`: the
//! calls arrive with the id, the name and the default look, and a refusal comes back as
//! [`RegisterFault::Refused`] once and ends the asking. No real bus, no real account service.

use super::*;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex, PoisonError};
use zbus::fdo;

/// A dbus-daemon on the repo's private-bus.conf with a cleared environment, killed and reaped
/// when dropped.
struct PrivateBus {
    daemon: Child,
    address: String,
}

impl PrivateBus {
    fn start() -> PrivateBus {
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
}

impl Drop for PrivateBus {
    fn drop(&mut self) {
        let _ = self.daemon.kill();
        let _ = self.daemon.wait();
    }
}

/// What the stand-in heard: (id, name, look) per call.
type Heard = Arc<Mutex<Vec<(String, String, String)>>>;

/// The stand-in `org.quire.Spaces1`: records each `Register`, and answers `AccessDenied` when it
/// was built to refuse.
struct StandIn {
    heard: Heard,
    refuse: bool,
}

#[zbus::interface(name = "org.quire.Spaces1")]
impl StandIn {
    fn register(&self, id: String, name: String, look: String) -> fdo::Result<()> {
        self.heard
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push((id, name, look));
        match self.refuse {
            true => Err(fdo::Error::AccessDenied("not the compositor".to_owned())),
            false => Ok(()),
        }
    }
}

/// The stand-in served on `address` by a runtime of its own; dropped, it goes away.
struct Served {
    _runtime: tokio::runtime::Runtime,
    _connection: zbus::Connection,
    heard: Heard,
}

fn serve(address: &str, refuse: bool) -> Served {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("runtime");
    let heard = Heard::default();
    let object = StandIn {
        heard: heard.clone(),
        refuse,
    };
    let connection = runtime
        .block_on(async {
            zbus::connection::Builder::address(address)?
                .name("org.quire.Accounts1")?
                .serve_at("/org/quire/Spaces1", object)?
                .build()
                .await
        })
        .expect("served");
    Served {
        _runtime: runtime,
        _connection: connection,
        heard,
    }
}

fn registration(id: &str, name: &str) -> SpaceRegistration {
    SpaceRegistration::new(SpaceId::parse(id).expect("an id"), name.to_owned())
}

/// Polls `ready` for up to five seconds (500 turns of 10 ms).
fn until(mut ready: impl FnMut() -> bool) -> bool {
    for _ in 0..500 {
        if ready() {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    false
}

#[test]
fn a_batch_reaches_the_service_with_the_default_look() {
    let bus = PrivateBus::start();
    let served = serve(&bus.address, false);
    let mut registry = ZbusSpaceRegistry::at(bus.address.clone());
    registry
        .register(vec![
            registration("s-1", "Desktop 1"),
            registration("s-2", "Desktop 2"),
        ])
        .expect("queued");
    assert!(until(|| {
        served
            .heard
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .len()
            == 2
    }));
    let heard = served
        .heard
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone();
    assert_eq!(
        heard,
        [
            ("s-1".to_owned(), "Desktop 1".to_owned(), String::new()),
            ("s-2".to_owned(), "Desktop 2".to_owned(), String::new()),
        ]
    );
    assert!(registry.failures().is_empty());
}

#[test]
fn a_refusal_is_reported_once_and_ends_the_asking() {
    let bus = PrivateBus::start();
    let served = serve(&bus.address, true);
    let mut registry = ZbusSpaceRegistry::at(bus.address.clone());
    registry
        .register(vec![
            registration("s-1", "Desktop 1"),
            registration("s-2", "Desktop 2"),
        ])
        .expect("queued");
    let mut failures = Vec::new();
    assert!(until(|| {
        failures.extend(registry.failures());
        !failures.is_empty()
    }));
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].fault, RegisterFault::Refused);
    assert_eq!(failures[0].id.as_str(), "s-1");
    registry
        .register(vec![registration("s-3", "Desktop 3")])
        .expect("queued");
    std::thread::sleep(std::time::Duration::from_millis(200));
    assert_eq!(
        served
            .heard
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .len(),
        1,
        "nothing is asked after a refusal"
    );
}

#[test]
fn no_service_on_the_bus_means_try_again_later() {
    let bus = PrivateBus::start();
    let mut registry = ZbusSpaceRegistry::at(bus.address.clone());
    registry
        .register(vec![registration("s-1", "Desktop 1")])
        .expect("queued");
    let mut failures = Vec::new();
    assert!(until(|| {
        failures.extend(registry.failures());
        !failures.is_empty()
    }));
    assert_eq!(failures[0].fault, RegisterFault::Later);
}
