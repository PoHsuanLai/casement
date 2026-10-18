//! Hearing logind's `PrepareForSleep` (P1-T): the thread at the binary's edge that tells the tty
//! session the machine is about to sleep, and that it woke.
//!
//! libseat does not report a suspend, so the compositor listens on the system bus itself. The
//! watcher holds a `sleep` delay inhibitor while the machine is awake; on `PrepareForSleep(true)`
//! it calls [`SleepHandler::starting`] (which returns once the compositor has let go of its
//! devices, or gave up waiting), then drops the inhibitor so logind lets the sleep go on; on
//! `PrepareForSleep(false)` it calls [`SleepHandler::resumed`] and takes the inhibitor again. The
//! compositor names no bus type: the one D-Bus crate is this one (A4).
//!
//! Only logind is believed (CASW-03): the system bus lets any process send any signal, so a
//! `PrepareForSleep` from anyone else would take the displays away from the session. The match
//! rule names the sender, which the bus resolves to whoever owns `org.freedesktop.login1`, and
//! each signal is checked again against that owner's unique name before it is acted on.
//!
//! ```no_run
//! use casement_ipc::sleep::{SleepBus, SleepHandler, SleepWatch};
//!
//! struct Say;
//! impl SleepHandler for Say {
//!     fn starting(&mut self) { println!("sleeping"); }
//!     fn resumed(&mut self) { println!("awake"); }
//! }
//! let _watch = SleepWatch::start(SleepBus::System, Say).expect("login1 is on the bus");
//! ```

use crate::stop_on_drop::StopOnDrop;
use std::future::poll_fn;
use std::pin::Pin;
use std::sync::mpsc;
use std::time::Duration;
use tokio::sync::oneshot;
use zbus::export::futures_core::Stream;
use zbus::message::Type;
use zbus::names::BusName;
use zbus::zvariant::OwnedFd;
use zbus::{Connection, MatchRule, Message, MessageStream, Proxy, fdo};

const LOGIND: &str = "org.freedesktop.login1";
const MANAGER: &str = "org.freedesktop.login1.Manager";
const MANAGER_PATH: &str = "/org/freedesktop/login1";

/// How long [`SleepWatch::start`] waits for the bus before giving up.
const CONNECT_WITHIN: Duration = Duration::from_secs(10);

/// Which bus login1 is on.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SleepBus {
    /// The system bus (a real seat).
    System,
    /// A bus at this address (a test's private bus, with the fake logind on it).
    At(String),
}

/// What the watcher tells the compositor.
pub trait SleepHandler: Send + 'static {
    /// The machine is about to sleep. Returns when the compositor has let go of its devices (or
    /// has stopped waiting); logind is held back until then.
    fn starting(&mut self);

    /// The machine woke.
    fn resumed(&mut self);
}

/// Why the watcher could not start.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SleepError {
    /// The thread could not be started.
    #[error("the sleep watcher thread: {0}")]
    Thread(#[source] std::io::Error),
    /// The bus could not be reached or refused the subscription.
    #[error("bus: {0}")]
    Bus(#[source] Box<zbus::Error>),
    /// The bus did not answer in time.
    #[error("the bus did not answer within {} seconds", CONNECT_WITHIN.as_secs())]
    Timeout,
}

impl From<zbus::Error> for SleepError {
    fn from(error: zbus::Error) -> SleepError {
        SleepError::Bus(Box::new(error))
    }
}

/// The running watcher; dropping it stops the thread and releases the inhibitor.
#[derive(Debug)]
pub struct SleepWatch {
    _guard: StopOnDrop,
}

impl SleepWatch {
    /// Connects to `bus`, subscribes to `PrepareForSleep` and takes the delay inhibitor. Returns
    /// once the subscription is made, so a signal sent after this is heard.
    pub fn start(bus: SleepBus, handler: impl SleepHandler) -> Result<SleepWatch, SleepError> {
        let (stop, stopped) = oneshot::channel();
        let (ready_tx, ready) = mpsc::channel();
        let thread = std::thread::Builder::new()
            .name("casement-sleep".to_owned())
            .spawn(move || run(bus, handler, stopped, ready_tx))
            .map_err(SleepError::Thread)?;
        match ready.recv_timeout(CONNECT_WITHIN) {
            Ok(Ok(())) => Ok(SleepWatch {
                _guard: StopOnDrop::new(stop, thread),
            }),
            Ok(Err(error)) => Err(error),
            Err(_) => Err(SleepError::Timeout),
        }
    }
}

/// The thread: a current-thread runtime, the connection, the subscription, the loop.
fn run(
    bus: SleepBus,
    mut handler: impl SleepHandler,
    mut stopped: oneshot::Receiver<()>,
    ready: mpsc::Sender<Result<(), SleepError>>,
) {
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            let _ = ready.send(Err(SleepError::Thread(error)));
            return;
        }
    };
    runtime.block_on(async move {
        let (connection, mut signals) = match subscribe(&bus).await {
            Ok(subscribed) => subscribed,
            Err(error) => {
                let _ = ready.send(Err(error.into()));
                return;
            }
        };
        // Held for its drop: closing the descriptor is what lets logind go on.
        let mut _inhibitor = take_inhibitor(&connection).await;
        let _ = ready.send(Ok(()));
        loop {
            let next = poll_fn(|cx| signals.as_mut().poll_next(cx));
            let message = tokio::select! {
                _ = &mut stopped => return,
                message = next => message,
            };
            let Some(Ok(message)) = message else {
                // The bus went away: nothing more will be heard.
                return;
            };
            if !from_logind(&connection, &message).await {
                continue;
            }
            // A signal whose body is not a boolean is not logind's and is not acted on.
            match message.body().deserialize::<bool>() {
                Ok(true) => {
                    // The handler blocks this thread (the runtime has nothing else to run)
                    // until the compositor is done; only then is logind let go on.
                    handler.starting();
                    _inhibitor = None;
                }
                Ok(false) => {
                    handler.resumed();
                    _inhibitor = take_inhibitor(&connection).await;
                }
                Err(_) => {}
            }
        }
    });
}

type Signals = Pin<Box<MessageStream>>;

/// Connects to `bus` and subscribes to `PrepareForSleep`.
async fn subscribe(bus: &SleepBus) -> zbus::Result<(Connection, Signals)> {
    let connection = match bus {
        SleepBus::System => Connection::system().await?,
        SleepBus::At(address) => {
            zbus::connection::Builder::address(address.as_str())?
                .build()
                .await?
        }
    };
    let rule = MatchRule::builder()
        .msg_type(Type::Signal)
        .sender(LOGIND)?
        .interface(MANAGER)?
        .member("PrepareForSleep")?
        .path(MANAGER_PATH)?
        .build();
    let stream = MessageStream::for_match_rule(rule, &connection, None).await?;
    Ok((connection, Box::pin(stream)))
}

/// Whether `message` was sent by logind itself: its sender is the unique name that owns
/// `org.freedesktop.login1` now. A message with no sender, or one whose owner cannot be
/// resolved, is not logind's.
async fn from_logind(connection: &Connection, message: &Message) -> bool {
    let Some(sender) = message.header().sender().map(ToString::to_string) else {
        return false;
    };
    let Ok(logind) = BusName::try_from(LOGIND) else {
        return false;
    };
    let Ok(dbus) = fdo::DBusProxy::new(connection).await else {
        return false;
    };
    match dbus.get_name_owner(logind).await {
        Ok(owner) => owner.as_str() == sender,
        Err(_) => false,
    }
}

/// The delay inhibitor, or none when logind refuses (the sleep then goes on without waiting).
async fn take_inhibitor(connection: &Connection) -> Option<OwnedFd> {
    let manager = Proxy::new(connection, LOGIND, MANAGER_PATH, MANAGER)
        .await
        .ok()?;
    manager
        .call(
            "Inhibit",
            &(
                "sleep",
                "casement",
                "give the displays back before sleep",
                "delay",
            ),
        )
        .await
        .ok()
}
