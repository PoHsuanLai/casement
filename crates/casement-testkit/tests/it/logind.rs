//! The fake logind (P1-F2): inhibitors held and released, `Lock` and `Unlock` signals,
//! `PrepareForSleep` held by inhibitors, the session and seat calls recorded. Everything runs
//! on a private dbus-daemon; the client side is a second connection to it.

use casement_ipc::sleep::{SleepBus, SleepHandler, SleepWatch};
use casement_testkit::fakes::{
    FakeLogind, IdleHint, LogindCall, LogindError, PauseKind, PrivateBus, SleepPhase,
};
use std::future::poll_fn;
use std::pin::Pin;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;
use tokio::runtime::{Builder, Runtime};
use zbus::export::futures_core::Stream;
use zbus::message::Type;
use zbus::zvariant::OwnedFd;
use zbus::{Connection, MatchRule, Message, MessageStream, Proxy};

const BUS_NAME: &str = "org.freedesktop.login1";
const MANAGER: &str = "org.freedesktop.login1.Manager";
const SESSION: &str = "org.freedesktop.login1.Session";
const SEAT: &str = "org.freedesktop.login1.Seat";
const MANAGER_PATH: &str = "/org/freedesktop/login1";
const SESSION_PATH: &str = "/org/freedesktop/login1/session/c1";
const SEAT_PATH: &str = "/org/freedesktop/login1/seat/seat0";
const IN_TIME: Duration = Duration::from_secs(5);

fn runtime() -> Runtime {
    Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime")
}

async fn connect(address: &str) -> Connection {
    zbus::connection::Builder::address(address)
        .expect("address")
        .build()
        .await
        .expect("client")
}

async fn proxy<'a>(client: &Connection, path: &'a str, interface: &'a str) -> Proxy<'a> {
    Proxy::new(client, BUS_NAME, path, interface)
        .await
        .expect("proxy")
}

async fn signals(client: &Connection, path: &str, interface: &str) -> Pin<Box<MessageStream>> {
    let rule = MatchRule::builder()
        .msg_type(Type::Signal)
        .path(path)
        .expect("path")
        .interface(interface)
        .expect("interface")
        .build();
    Box::pin(
        MessageStream::for_match_rule(rule, client, None)
            .await
            .expect("stream"),
    )
}

/// The next signal on `stream`: its member name and the message.
async fn next_signal(stream: &mut Pin<Box<MessageStream>>) -> (String, Message) {
    let next = poll_fn(|cx| stream.as_mut().poll_next(cx));
    let message = tokio::time::timeout(IN_TIME, next)
        .await
        .expect("a signal in time")
        .expect("stream open")
        .expect("message");
    let member = message
        .header()
        .member()
        .map(|member| member.as_str().to_owned())
        .expect("member");
    (member, message)
}

async fn inhibit(manager: &Proxy<'_>, what: &str) -> OwnedFd {
    manager
        .call("Inhibit", &(what, "casement", "test", "delay"))
        .await
        .expect("Inhibit")
}

#[test]
fn an_inhibitor_is_held_until_its_pipe_closes() {
    let bus = PrivateBus::start().expect("bus");
    let logind = FakeLogind::serve(bus.address()).expect("serve");
    let rt = runtime();
    // Connections and streams drop at the end of the test, and dropping one spawns on the runtime.
    let _in_runtime = rt.enter();
    let fd = rt.block_on(async {
        let client = connect(bus.address()).await;
        inhibit(&proxy(&client, MANAGER_PATH, MANAGER).await, "sleep").await
    });
    assert_eq!(logind.held_inhibitors(), ["sleep"]);
    drop(fd);
    logind.wait_released(IN_TIME).expect("released");
    assert!(logind.held_inhibitors().is_empty());
    assert_eq!(
        logind.calls(),
        [
            LogindCall::Inhibit("sleep".to_owned()),
            LogindCall::InhibitReleased("sleep".to_owned()),
        ]
    );
}

#[test]
fn lock_and_unlock_signals_reach_a_listener() {
    let bus = PrivateBus::start().expect("bus");
    let logind = FakeLogind::serve(bus.address()).expect("serve");
    let rt = runtime();
    // Connections and streams drop at the end of the test, and dropping one spawns on the runtime.
    let _in_runtime = rt.enter();
    let (client, mut heard) = rt.block_on(async {
        let client = connect(bus.address()).await;
        let heard = signals(&client, SESSION_PATH, SESSION).await;
        (client, heard)
    });
    logind.lock().expect("lock");
    logind.unlock().expect("unlock");
    let members = rt.block_on(async {
        let (first, _) = next_signal(&mut heard).await;
        let (second, _) = next_signal(&mut heard).await;
        [first, second]
    });
    assert_eq!(members, ["Lock", "Unlock"]);
    drop(client);
}

#[test]
fn prepare_for_sleep_waits_for_inhibitors() {
    let bus = PrivateBus::start().expect("bus");
    let logind = FakeLogind::serve(bus.address()).expect("serve");
    let rt = runtime();
    // Connections and streams drop at the end of the test, and dropping one spawns on the runtime.
    let _in_runtime = rt.enter();
    let (client, mut heard, fd) = rt.block_on(async {
        let client = connect(bus.address()).await;
        let heard = signals(&client, MANAGER_PATH, MANAGER).await;
        let fd = inhibit(&proxy(&client, MANAGER_PATH, MANAGER).await, "sleep").await;
        (client, heard, fd)
    });
    logind
        .prepare_for_sleep(SleepPhase::Starting)
        .expect("starting");
    let going_down = rt.block_on(async {
        let (member, message) = next_signal(&mut heard).await;
        (member, message.body().deserialize::<bool>().expect("body"))
    });
    assert_eq!(going_down, ("PrepareForSleep".to_owned(), true));
    // The inhibitor is still held, so the sleep may not go on.
    let held = logind.wait_released(Duration::from_millis(100));
    assert!(
        matches!(&held, Err(LogindError::StillHeld { what }) if what == &["sleep"]),
        "{held:?}"
    );
    drop(fd);
    logind.wait_released(IN_TIME).expect("released");
    logind
        .prepare_for_sleep(SleepPhase::Resumed)
        .expect("resumed");
    let woke = rt.block_on(async {
        let (_, message) = next_signal(&mut heard).await;
        message.body().deserialize::<bool>().expect("body")
    });
    assert!(!woke);
    drop(client);
}

#[test]
fn session_and_seat_calls_are_recorded_and_device_signals_sent() {
    let bus = PrivateBus::start().expect("bus");
    let logind = FakeLogind::serve(bus.address()).expect("serve");
    let rt = runtime();
    // Connections and streams drop at the end of the test, and dropping one spawns on the runtime.
    let _in_runtime = rt.enter();
    let (client, mut heard) = rt.block_on(async {
        let client = connect(bus.address()).await;
        let heard = signals(&client, SESSION_PATH, SESSION).await;
        let session = proxy(&client, SESSION_PATH, SESSION).await;
        let seat = proxy(&client, SEAT_PATH, SEAT).await;
        let manager = proxy(&client, MANAGER_PATH, MANAGER).await;
        let path: zbus::zvariant::OwnedObjectPath = manager
            .call("GetSession", &("auto",))
            .await
            .expect("GetSession");
        assert_eq!(path.as_str(), SESSION_PATH);
        let (): () = session.call("TakeControl", &(false,)).await.expect("take");
        let (_device, paused): (OwnedFd, bool) = session
            .call("TakeDevice", &(226u32, 0u32))
            .await
            .expect("TakeDevice");
        assert!(!paused);
        let (): () = session
            .call("SetIdleHint", &(true,))
            .await
            .expect("SetIdleHint");
        let (): () = session
            .call("PauseDeviceComplete", &(226u32, 0u32))
            .await
            .expect("PauseDeviceComplete");
        let (): () = session
            .call("ReleaseDevice", &(226u32, 0u32))
            .await
            .expect("ReleaseDevice");
        let (): () = session.call("ReleaseControl", &()).await.expect("release");
        let (): () = seat.call("SwitchTo", &(3u32,)).await.expect("SwitchTo");
        (client, heard)
    });
    assert_eq!(
        logind.calls(),
        [
            LogindCall::TakeControl,
            LogindCall::TakeDevice(226, 0),
            LogindCall::IdleHint(IdleHint::Idle),
            LogindCall::PauseDeviceComplete(226, 0),
            LogindCall::ReleaseDevice(226, 0),
            LogindCall::ReleaseControl,
            LogindCall::SwitchTo(3),
        ]
    );
    logind
        .pause_device(226, 0, PauseKind::Pause)
        .expect("pause");
    logind.resume_device(226, 0).expect("resume");
    let heard_members = rt.block_on(async {
        let (paused, message) = next_signal(&mut heard).await;
        let body: (u32, u32, String) = message.body().deserialize().expect("body");
        let (resumed, _) = next_signal(&mut heard).await;
        (paused, body, resumed)
    });
    assert_eq!(
        heard_members,
        (
            "PauseDevice".to_owned(),
            (226, 0, "pause".to_owned()),
            "ResumeDevice".to_owned()
        )
    );
    drop(client);
}

/// What a sleep watcher told its handler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Told {
    Starting,
    Resumed,
}

/// A handler that writes down what it is told.
struct Record(Arc<Mutex<Vec<Told>>>);

impl Record {
    fn note(&self, told: Told) {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(told);
    }
}

impl SleepHandler for Record {
    fn starting(&mut self) {
        self.note(Told::Starting);
    }

    fn resumed(&mut self) {
        self.note(Told::Resumed);
    }
}

/// CASW-03: the bus lets any process send `PrepareForSleep`; only logind's is acted on. A second
/// connection claims the machine is going to sleep, and the watcher's handler hears nothing of
/// it, while the real logind's signal that follows is heard.
#[test]
fn a_prepare_for_sleep_from_anyone_but_logind_is_not_acted_on() {
    let bus = PrivateBus::start().expect("bus");
    let logind = FakeLogind::serve(bus.address()).expect("serve");
    let told = Arc::new(Mutex::new(Vec::new()));
    let _watch = SleepWatch::start(
        SleepBus::At(bus.address().to_owned()),
        Record(Arc::clone(&told)),
    )
    .expect("the watcher subscribes");
    let rt = runtime();
    // Connections and streams drop at the end of the test, and dropping one spawns on the runtime.
    let _in_runtime = rt.enter();
    let spoofer = rt.block_on(async {
        let spoofer = connect(bus.address()).await;
        spoofer
            .emit_signal(
                None::<&str>,
                MANAGER_PATH,
                MANAGER,
                "PrepareForSleep",
                &(true,),
            )
            .await
            .expect("the spoofed signal is sent");
        // A round trip on the same connection: the bus has handled the signal before this
        // returns, so it reached the watcher before anything logind sends next.
        zbus::fdo::DBusProxy::new(&spoofer)
            .await
            .expect("the bus proxy")
            .get_id()
            .await
            .expect("the bus answers");
        spoofer
    });
    logind
        .prepare_for_sleep(SleepPhase::Resumed)
        .expect("logind's own signal");
    // Counted waits, not the clock: only casement/src/time/ reads one (boundary B8).
    const STEP: Duration = Duration::from_millis(10);
    let heard = (0..IN_TIME.as_millis() / STEP.as_millis()).any(|_| {
        let heard = told
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .contains(&Told::Resumed);
        if !heard {
            std::thread::sleep(STEP);
        }
        heard
    });
    assert!(heard, "logind's signal was not heard");
    assert_eq!(
        *told.lock().unwrap_or_else(PoisonError::into_inner),
        [Told::Resumed],
        "the spoofed sleep was not acted on"
    );
    drop(spoofer);
}
