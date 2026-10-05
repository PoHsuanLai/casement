//! `serve_debug_at` over a private dbus-daemon: a casementctl-style client call reaches the
//! calloop channel and the answer the loop sends comes back.

use super::*;
use crate::dbus::debug1::tests::PrivateBus;
use crate::dbus::{DebugReply, DebugRequest};
use crate::{InjectLine, WaitPredicate};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

/// A calloop loop on its own thread standing in for casement's: it records each request and
/// answers each call the way casement would in shape.
struct StandInLoop {
    seen: Arc<Mutex<Vec<DebugRequest>>>,
    stop: calloop::LoopSignal,
    thread: Option<JoinHandle<()>>,
}

impl StandInLoop {
    fn run(channel: Channel<DebugCall>) -> StandInLoop {
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
                        let reply = match &call.request {
                            DebugRequest::Windows => DebugReply::Json("[]".to_owned()),
                            DebugRequest::Wait(..) => {
                                DebugReply::Waited("met".to_owned(), "{}".to_owned())
                            }
                            DebugRequest::Inject(lines) => DebugReply::Injected(
                                lines.iter().map(|_| "ok".to_owned()).collect(),
                            ),
                            _ => DebugReply::Done,
                        };
                        record.lock().expect("seen").push(call.request);
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

    fn requests(&self) -> Vec<DebugRequest> {
        self.seen.lock().expect("seen").clone()
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

async fn client(address: &str) -> zbus::Proxy<'static> {
    let connection = zbus::connection::Builder::address(address)
        .expect("address")
        .build()
        .await
        .expect("client");
    zbus::Proxy::new(
        &connection,
        Debug1::BUS_NAME,
        OBJECT_PATH,
        "org.quire.Casement.Debug1",
    )
    .await
    .expect("proxy")
}

#[tokio::test]
async fn a_client_call_reaches_the_channel_and_gets_the_reply() {
    let bus = PrivateBus::start();
    let (channel, _connection) = serve_debug_at(&bus.address).expect("serve");
    let event_loop = StandInLoop::run(channel);
    let proxy = client(&bus.address).await;

    let windows: String = proxy.call("Windows", &()).await.expect("Windows");
    assert_eq!(windows, "[]");
    let (outcome, scene): (String, String) = proxy
        .call("Wait", &("settled", 250_u32))
        .await
        .expect("Wait");
    assert_eq!((outcome.as_str(), scene.as_str()), ("met", "{}"));
    let injected: Vec<String> = proxy
        .call("Inject", &(vec!["sleep 5"],))
        .await
        .expect("Inject");
    assert_eq!(injected, ["ok"]);

    let seen = event_loop.requests();
    assert_eq!(seen.len(), 3, "{seen:?}");
    assert_eq!(seen[0], DebugRequest::Windows);
    assert!(matches!(
        seen[1],
        DebugRequest::Wait(WaitPredicate::Settled, _)
    ));
    assert!(
        matches!(&seen[2], DebugRequest::Inject(lines) if matches!(lines[..], [InjectLine::Sleep(_)]))
    );
}

#[tokio::test]
async fn the_name_is_owned_by_the_connection_serve_debug_returns() {
    let bus = PrivateBus::start();
    let (_channel, connection) = serve_debug_at(&bus.address).expect("serve");
    let owner = zbus::fdo::DBusProxy::new(&connection)
        .await
        .expect("dbus proxy")
        .get_name_owner(Debug1::BUS_NAME.try_into().expect("name"))
        .await
        .expect("owner");
    assert_eq!(
        Some(owner.as_str()),
        connection.unique_name().map(|n| n.as_str())
    );
}

#[test]
fn a_second_server_on_the_same_bus_is_refused_and_a_dead_bus_is_an_error() {
    let bus = PrivateBus::start();
    let first = serve_debug_at(&bus.address).expect("serve");
    let second = serve_debug_at(&bus.address);
    assert!(matches!(second, Err(IpcError::Bus(_))), "{second:?}");
    drop(first);
    let nowhere = serve_debug_at("unix:path=/nonexistent/casement-debug1-test");
    assert!(matches!(nowhere, Err(IpcError::Bus(_))), "{nowhere:?}");
}
