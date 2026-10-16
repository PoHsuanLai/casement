//! `serve_compositor_at` over a private dbus-daemon: a `Connect` call reaches the channel with
//! the caller's control group read through the seam, and the stand-in loop's answer comes back
//! as the reply or the right D-Bus error. A table of fake control groups stands in for `/proc`.

use super::*;
use crate::dbus::{ConnectAsk, ConnectRefusal, ConnectReply};
use casement_core::policy::ConnectionRole;
use std::io::{BufRead, BufReader};
use std::os::unix::net::UnixStream;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::JoinHandle;

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

/// The control group text every process reads as; `None` is a process the table does not know.
#[derive(Debug, Default)]
struct Table(Mutex<Option<String>>);

impl Table {
    fn runs_in(&self, text: Option<&str>) {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) = text.map(str::to_owned);
    }
}

impl PeerCgroups for Table {
    fn cgroup_of(&self, pid: u32) -> Result<String, IpcError> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
            .ok_or_else(|| IpcError::Bus(format!("no process {pid}")))
    }
}

/// A calloop loop on its own thread standing in for casement's: it records each ask, grants a
/// socket to a caller whose cgroup names `sill.service` asking for the shell, and refuses the
/// rest the way `casement_agent::hand_out` would.
struct StandInLoop {
    seen: Arc<Mutex<Vec<ConnectAsk>>>,
    stop: calloop::LoopSignal,
    thread: Option<JoinHandle<()>>,
}

impl StandInLoop {
    fn run(channel: Channel<ConnectCall>) -> StandInLoop {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let record = seen.clone();
        let (signals, signal) = std::sync::mpsc::channel();
        let thread = std::thread::spawn(move || {
            let mut event_loop = calloop::EventLoop::<()>::try_new().expect("loop");
            signals.send(event_loop.get_signal()).expect("signal");
            event_loop
                .handle()
                .insert_source(channel, move |event, (), ()| {
                    if let calloop::channel::Event::Msg(call) = event {
                        let reply = decide(&call.ask);
                        record.lock().expect("seen").push(call.ask);
                        let _ = call.reply.send(reply);
                    }
                })
                .expect("source");
            event_loop.run(None, &mut (), |()| {}).expect("run");
        });
        StandInLoop {
            seen,
            stop: signal.recv().expect("signal"),
            thread: Some(thread),
        }
    }

    fn asks(&self) -> Vec<ConnectAsk> {
        self.seen.lock().expect("seen").clone()
    }
}

fn decide(ask: &ConnectAsk) -> ConnectReply {
    if ask.end.is_some() {
        let portal = ask.caller_cgroup.trim_end().ends_with("/transom.service");
        return match portal {
            true => ConnectReply::Ended,
            false => ConnectReply::Refused(ConnectRefusal::UnknownUnit),
        };
    }
    if ask.remote.is_some() {
        let portal = ask.caller_cgroup.trim_end().ends_with("/transom.service");
        return match (portal, UnixStream::pair()) {
            (true, Ok((_server, client))) => ConnectReply::Granted(client.into()),
            (true, Err(_)) => ConnectReply::Refused(ConnectRefusal::Unavailable),
            (false, _) => ConnectReply::Refused(ConnectRefusal::UnknownUnit),
        };
    }
    let sill = ask.caller_cgroup.trim_end().ends_with("/sill.service");
    match (sill, &ask.role) {
        (true, ConnectionRole::Shell) => match UnixStream::pair() {
            Ok((_server, client)) => ConnectReply::Granted(client.into()),
            Err(_) => ConnectReply::Refused(ConnectRefusal::Unavailable),
        },
        (true, _) => ConnectReply::Refused(ConnectRefusal::RoleNotAllowed),
        (false, _) => ConnectReply::Refused(ConnectRefusal::UnknownUnit),
    }
}

impl Drop for StandInLoop {
    fn drop(&mut self) {
        self.stop.stop();
        self.stop.wakeup();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

async fn proxy(address: &str) -> zbus::Proxy<'static> {
    let connection = zbus::connection::Builder::address(address)
        .expect("address")
        .build()
        .await
        .expect("client");
    zbus::Proxy::new(&connection, BUS_NAME, OBJECT_PATH, "org.quire.Compositor1")
        .await
        .expect("proxy")
}

/// What `Connect(role)` came to: the error's D-Bus name, or `granted`.
async fn connect_as(proxy: &zbus::Proxy<'_>, role: &str) -> String {
    match proxy
        .call::<_, _, zbus::zvariant::OwnedFd>("Connect", &(role,))
        .await
    {
        Ok(_) => "granted".to_owned(),
        Err(zbus::Error::MethodError(name, ..)) => name.to_string(),
        Err(other) => format!("{other}"),
    }
}

#[tokio::test]
async fn connect_is_decided_by_the_callers_unit_over_a_private_bus() {
    let bus = PrivateBus::start();
    let cgroups = Arc::new(Table::default());
    let (channel, _connection) = serve_compositor_at(&bus.address, cgroups.clone()).expect("serve");
    let event_loop = StandInLoop::run(channel);
    let proxy = proxy(&bus.address).await;

    let sill = "0::/user.slice/user@1000.service/app.slice/sill.service\n";
    let firefox = "0::/user.slice/user@1000.service/app.slice/firefox.service\n";
    // (case, the caller's cgroup, the role named, what comes back)
    let cases = [
        (
            "the shell's unit asks for the shell",
            Some(sill),
            "shell",
            "granted",
        ),
        (
            "the shell's unit asks for the agent tier",
            Some(sill),
            "agent",
            "org.freedesktop.DBus.Error.AccessDenied",
        ),
        (
            "another program asks for the shell",
            Some(firefox),
            "shell",
            "org.freedesktop.DBus.Error.AccessDenied",
        ),
        (
            "a role casement does not hand out",
            Some(sill),
            "public",
            "org.freedesktop.DBus.Error.InvalidArgs",
        ),
        (
            "a caller whose process is not known",
            None,
            "shell",
            "org.freedesktop.DBus.Error.Failed",
        ),
    ];
    for (case, cgroup, role, want) in cases {
        cgroups.runs_in(cgroup);
        assert_eq!(connect_as(&proxy, role).await, want, "{case}");
    }
    let asks = event_loop.asks();
    assert_eq!(
        asks.len(),
        3,
        "only well-formed asks from known callers reach the loop"
    );
    assert_eq!(asks[0].role, ConnectionRole::Shell);
    assert_eq!(asks[0].caller_cgroup, sill);
}

/// What `ConnectEis(session, devices)` came to: the error's D-Bus name, or `granted`.
async fn connect_eis_as(proxy: &zbus::Proxy<'_>, session: u64, devices: u32) -> String {
    match proxy
        .call::<_, _, zbus::zvariant::OwnedFd>("ConnectEis", &(session, devices))
        .await
    {
        Ok(_) => "granted".to_owned(),
        Err(zbus::Error::MethodError(name, ..)) => name.to_string(),
        Err(other) => format!("{other}"),
    }
}

#[tokio::test]
async fn connect_eis_reaches_the_loop_as_a_remote_ask_with_the_callers_unit() {
    let bus = PrivateBus::start();
    let cgroups = Arc::new(Table::default());
    let (channel, _connection) = serve_compositor_at(&bus.address, cgroups.clone()).expect("serve");
    let event_loop = StandInLoop::run(channel);
    let proxy = proxy(&bus.address).await;

    let portal = "0::/user.slice/user@1000.service/app.slice/transom.service\n";
    let firefox = "0::/user.slice/user@1000.service/app.slice/firefox.service\n";
    cgroups.runs_in(Some(portal));
    assert_eq!(connect_eis_as(&proxy, 7, 3).await, "granted");
    cgroups.runs_in(Some(firefox));
    assert_eq!(
        connect_eis_as(&proxy, 8, 7).await,
        "org.freedesktop.DBus.Error.AccessDenied"
    );
    let asks = event_loop.asks();
    assert_eq!(asks.len(), 2);
    assert_eq!(
        asks[0].remote,
        Some(crate::dbus::RemoteAsk {
            session: 7,
            devices: 3
        })
    );
    assert_eq!(asks[0].caller_cgroup, portal);
}

/// What `EndEis(session)` came to: the error's D-Bus name, or `ended`.
async fn end_eis_as(proxy: &zbus::Proxy<'_>, session: u64) -> String {
    match proxy.call::<_, _, ()>("EndEis", &(session,)).await {
        Ok(()) => "ended".to_owned(),
        Err(zbus::Error::MethodError(name, ..)) => name.to_string(),
        Err(other) => format!("{other}"),
    }
}

/// PORTAL-05: `EndEis` is checked against the caller's unit exactly as `ConnectEis` is, and
/// reaches the loop as an end ask for the session named.
#[tokio::test]
async fn end_eis_reaches_the_loop_only_from_the_portals_unit() {
    let bus = PrivateBus::start();
    let cgroups = Arc::new(Table::default());
    let (channel, _connection) = serve_compositor_at(&bus.address, cgroups.clone()).expect("serve");
    let event_loop = StandInLoop::run(channel);
    let proxy = proxy(&bus.address).await;

    let portal = "0::/user.slice/user@1000.service/app.slice/transom.service\n";
    let firefox = "0::/user.slice/user@1000.service/app.slice/firefox.service\n";
    cgroups.runs_in(Some(portal));
    assert_eq!(end_eis_as(&proxy, 7).await, "ended");
    cgroups.runs_in(Some(firefox));
    assert_eq!(
        end_eis_as(&proxy, 8).await,
        "org.freedesktop.DBus.Error.AccessDenied"
    );
    let asks = event_loop.asks();
    assert_eq!(asks.len(), 2);
    assert_eq!(asks[0].end, Some(crate::dbus::EndAsk { session: 7 }));
    assert_eq!(asks[0].remote, None, "an end is not a connect");
    assert_eq!(asks[0].caller_cgroup, portal);
}
