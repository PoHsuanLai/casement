//! Compositor1 served on a private bus, for the tests of this crate: every caller is made to
//! stand as a chosen program (`BusPeers::introduce`), and a stand-in event loop answers what
//! gets through the caller check. Nothing here reads the real `/proc` for a caller.

use crate::dbus::{
    COMPOSITOR_BUS_NAME as BUS_NAME, COMPOSITOR_OBJECT_PATH as OBJECT_PATH, CallerSetup,
    ConnectAsk, ConnectCall, ConnectReply, serve_compositor_at,
};
use crate::privacy::PrivacyLink;
use crate::test_bus::{PrivateBus, name_of};
use calloop::channel::Channel;
use peephole_bus::peephole::fake::introduced_service;
use peephole_bus::peephole_core::{Pin, Proof, Standing, Table};
use peephole_bus::{BusPeers, ManagerCheck};
use std::os::unix::net::UnixStream;
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::JoinHandle;
use std::time::Duration;

pub(crate) const DENIED: &str = "org.freedesktop.DBus.Error.AccessDenied";
pub(crate) const TOO_MANY: &str = "org.freedesktop.DBus.Error.LimitsExceeded";
pub(crate) const BAD_ARGS: &str = "org.freedesktop.DBus.Error.InvalidArgs";

/// A window so long that the rate limit counts one for the whole test.
pub(crate) const WHOLE_TEST: Duration = Duration::from_secs(3600);

/// The check the bus thread makes, reachable from the test to make senders stand as it likes.
type Peers = BusPeers;

/// A calloop loop on its own thread standing in for casement's: it records each ask and grants
/// a socket to every ask it is given (the bus thread has already refused the rest).
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
        return ConnectReply::Ended;
    }
    match UnixStream::pair() {
        Ok((_server, client)) => ConnectReply::Granted(client.into()),
        Err(_) => ConnectReply::Refused(crate::dbus::ConnectRefusal::Unavailable),
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

/// Compositor1 served on a private bus.
pub(crate) struct Served {
    pub(crate) bus: PrivateBus,
    peers: Arc<OnceLock<Arc<Peers>>>,
    /// Held so the name is, and so the channel of an undrained run stays open.
    _connection: zbus::Connection,
    drained: Option<StandInLoop>,
    _undrained: Option<Channel<ConnectCall>>,
}

/// Whether the stand-in loop reads the channel.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Loop {
    Reads,
    Stalled,
}

impl Served {
    /// Compositor1 on a new private bus, counting calls per `window`, with the Privacy list
    /// served too when `privacy` is given.
    pub(crate) fn start(
        window: Duration,
        event_loop: Loop,
        privacy: Option<PrivacyLink>,
    ) -> Served {
        let bus = PrivateBus::start();
        let peers: Arc<OnceLock<Arc<Peers>>> = Arc::default();
        let slot = Arc::clone(&peers);
        let setup = CallerSetup::with(move |connection| {
            let made = Arc::new(BusPeers::new(
                connection.clone(),
                Table::empty(),
                ManagerCheck::Unchecked,
            ));
            let _ = slot.set(Arc::clone(&made));
            made
        })
        .window(window);
        let setup = match privacy {
            Some(link) => setup.with_privacy(link),
            None => setup,
        };
        let (channel, connection) = serve_compositor_at(&bus.address, setup).expect("serve");
        let (drained, undrained) = match event_loop {
            Loop::Reads => (Some(StandInLoop::run(channel)), None),
            Loop::Stalled => (None, Some(channel)),
        };
        Served {
            bus,
            peers,
            _connection: connection,
            drained,
            _undrained: undrained,
        }
    }

    pub(crate) fn asks(&self) -> Vec<ConnectAsk> {
        self.drained
            .as_ref()
            .map(StandInLoop::asks)
            .unwrap_or_default()
    }

    /// A new client that stands as `standing`, with a proxy on Compositor1.
    pub(crate) async fn caller(&self, standing: Standing) -> zbus::Proxy<'static> {
        let client = self.bus.client().await;
        self.peers
            .get()
            .expect("served")
            .introduce(&name_of(&client), standing);
        zbus::Proxy::new(&client, BUS_NAME, OBJECT_PATH, "org.quire.Compositor1")
            .await
            .expect("proxy")
    }
}

/// A service of `role` proven up to `proof`.
pub(crate) fn service(role: &str, proof: Proof) -> Standing {
    introduced_service("org.quire.Test", role, proof, Pin::Reopened).expect("a standing")
}

/// What a call came to: `granted`, or the error's D-Bus name.
pub(crate) fn outcome<T>(result: zbus::Result<T>) -> String {
    match result {
        Ok(_) => "granted".to_owned(),
        Err(zbus::Error::MethodError(name, ..)) => name.to_string(),
        Err(other) => format!("{other}"),
    }
}
