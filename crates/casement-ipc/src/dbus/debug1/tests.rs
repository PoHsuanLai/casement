//! Debug1 against a stand-in event loop: the calls it forwards, the answers it maps, and one
//! round trip over a private dbus-daemon (the wire types the interface macro derives).

use super::*;
use crate::test_bus::PrivateBus;
use casement_core::geometry::LogicalPoint;
use casement_core::input::{ButtonCode, Keysym};
use casement_protocols::vocab::PressEdge;
use std::thread::JoinHandle;

type Answer = Box<dyn Fn(&DebugRequest) -> DebugReply + Send>;

/// A calloop loop on its own thread that answers every call with `answer` and, once the sender
/// is gone, returns the requests it saw.
fn stand_in(answer: Answer) -> (Debug1, JoinHandle<Vec<DebugRequest>>) {
    let (calls, channel) = calloop::channel::channel::<DebugCall>();
    let thread = std::thread::spawn(move || {
        let mut event_loop = calloop::EventLoop::<Vec<DebugRequest>>::try_new().expect("loop");
        let signal = event_loop.get_signal();
        event_loop
            .handle()
            .insert_source(channel, move |event, (), seen| match event {
                calloop::channel::Event::Msg(call) => {
                    let reply = answer(&call.request);
                    seen.push(call.request);
                    let _ = call.reply.send(reply);
                }
                calloop::channel::Event::Closed => signal.stop(),
            })
            .expect("source");
        let mut seen = Vec::new();
        event_loop.run(None, &mut seen, |_| {}).expect("run");
        seen
    });
    (Debug1::new(calls), thread)
}

fn echo() -> Answer {
    Box::new(|request| match request {
        DebugRequest::Inject(lines) => {
            DebugReply::Injected(lines.iter().map(|l| format!("ok {l:?}")).collect())
        }
        DebugRequest::Wait(..) => DebugReply::Waited("met".to_owned(), "{}".to_owned()),
        DebugRequest::Output(verb, _) if verb == "list" => DebugReply::Json("[]".to_owned()),
        DebugRequest::Windows | DebugRequest::Scene => DebugReply::Json("{}".to_owned()),
        _ => DebugReply::Done,
    })
}

#[tokio::test]
async fn inject_parses_every_line_then_forwards_them_in_order() {
    let (debug, seen) = stand_in(echo());
    let answers = debug
        .inject(vec![
            "move 3 4".to_owned(),
            "press left  ".to_owned(),
            " key a up".to_owned(),
        ])
        .await
        .expect("inject");
    assert_eq!(
        answers,
        ["ok move 3 4", "ok press left", "ok key a up"],
        "`ok <line>` as the grammar documents it, not the parsed line's Debug text"
    );
    drop(debug);
    assert_eq!(
        seen.join().expect("loop"),
        vec![DebugRequest::Inject(vec![
            InjectLine::Move(LogicalPoint { x: 3, y: 4 }),
            InjectLine::Press(ButtonCode(0x110)),
            InjectLine::Key(Keysym(0x61), PressEdge::Release),
        ])]
    );
}

#[tokio::test]
async fn a_bad_line_injects_nothing_and_names_its_number() {
    let (debug, seen) = stand_in(echo());
    let error = debug
        .inject(vec!["move 3 4".to_owned(), "wiggle".to_owned()])
        .await
        .expect_err("refused");
    assert!(
        matches!(&error, fdo::Error::InvalidArgs(why) if why.starts_with("line 2")),
        "{error}"
    );
    drop(debug);
    assert!(seen.join().expect("loop").is_empty());
}

#[tokio::test]
async fn the_rest_of_the_calls_forward_and_map_their_answers() {
    let (debug, seen) = stand_in(echo());
    assert_eq!(
        debug
            .wait("mapped foot".to_owned(), 1500)
            .await
            .expect("wait"),
        ("met".to_owned(), "{}".to_owned())
    );
    debug.clock("step".to_owned(), 3).await.expect("step");
    debug
        .clock("advance".to_owned(), 250)
        .await
        .expect("advance");
    assert_eq!(
        debug
            .output("list".to_owned(), Vec::new())
            .await
            .expect("list"),
        "[]"
    );
    assert_eq!(
        debug
            .output("remove".to_owned(), vec!["HEADLESS-2".to_owned()])
            .await
            .expect("remove"),
        ""
    );
    assert_eq!(debug.windows().await.expect("windows"), "{}");
    assert_eq!(debug.scene().await.expect("scene"), "{}");
    debug
        .fault("drop-output".to_owned(), 1)
        .await
        .expect("fault");
    drop(debug);
    let seen = seen.join().expect("loop");
    assert_eq!(seen.len(), 8);
    assert_eq!(
        seen[0],
        DebugRequest::Wait(
            WaitPredicate::Mapped("foot".to_owned()),
            Duration::from_millis(1500)
        )
    );
    assert_eq!(seen[1], DebugRequest::Step(3));
    assert_eq!(seen[2], DebugRequest::Advance(Duration::from_millis(250)));
    assert_eq!(seen[7], DebugRequest::Fault("drop-output".to_owned(), 1));
}

#[tokio::test]
async fn a_refusal_from_the_loop_is_a_failed_call() {
    let (debug, seen) = stand_in(Box::new(|_| DebugReply::Refused("wall clock".to_owned())));
    let error = debug
        .clock("step".to_owned(), 1)
        .await
        .expect_err("refused");
    assert!(
        matches!(&error, fdo::Error::Failed(why) if why == "wall clock"),
        "{error}"
    );
    let error = debug.windows().await.expect_err("refused");
    assert!(matches!(error, fdo::Error::Failed(_)));
    drop(debug);
    seen.join().expect("loop");
}

#[tokio::test]
async fn a_loop_that_answers_the_wrong_thing_or_is_gone_is_an_error_not_a_panic() {
    let (debug, seen) = stand_in(Box::new(|_| DebugReply::Done));
    assert!(debug.windows().await.is_err(), "Done is not JSON");
    drop(debug);
    seen.join().expect("loop");

    let (calls, channel) = calloop::channel::channel::<DebugCall>();
    drop(channel);
    let gone = Debug1::new(calls);
    assert!(gone.scene().await.is_err(), "no loop to ask");
}

#[tokio::test]
async fn malformed_calls_are_refused_before_the_loop_sees_them() {
    let (debug, seen) = stand_in(echo());
    let strs = |words: &[&str]| words.iter().map(|w| (*w).to_owned()).collect::<Vec<_>>();
    for result in [
        debug.wait("soon".to_owned(), 10).await.map(drop),
        debug.clock("rewind".to_owned(), 1).await,
        debug.fault("explode".to_owned(), 1).await,
        debug
            .output("list".to_owned(), strs(&["extra"]))
            .await
            .map(drop),
        debug
            .output("add".to_owned(), strs(&["DP-1"]))
            .await
            .map(drop),
        debug
            .output("remove".to_owned(), strs(&["bad name"]))
            .await
            .map(drop),
        debug
            .output("set".to_owned(), strs(&["DP-1"]))
            .await
            .map(drop),
        debug
            .output("set".to_owned(), strs(&["DP-1", "scale"]))
            .await
            .map(drop),
        debug
            .output("rotate".to_owned(), Vec::new())
            .await
            .map(drop),
    ] {
        assert!(
            matches!(result, Err(fdo::Error::InvalidArgs(_))),
            "{result:?}"
        );
    }
    let well_formed = [
        debug
            .output("add".to_owned(), strs(&["DP-2:1280x720@60000:120:1920,0"]))
            .await,
        debug
            .output("set".to_owned(), strs(&["DP-2", "scale=180", "power=off"]))
            .await,
    ];
    assert!(well_formed.iter().all(Result::is_ok), "{well_formed:?}");
    drop(debug);
    assert_eq!(seen.join().expect("loop").len(), 2);
}

#[tokio::test]
async fn the_interface_answers_over_a_private_bus() {
    let bus = PrivateBus::start();
    let (debug, _seen) = stand_in(echo());
    let _server = zbus::connection::Builder::address(bus.address.as_str())
        .expect("address")
        .name(Debug1::BUS_NAME)
        .expect("name")
        .serve_at(OBJECT_PATH, debug)
        .expect("serve")
        .build()
        .await
        .expect("server");
    let client = zbus::connection::Builder::address(bus.address.as_str())
        .expect("address")
        .build()
        .await
        .expect("client");
    let proxy = zbus::Proxy::new(
        &client,
        Debug1::BUS_NAME,
        OBJECT_PATH,
        "org.quire.Casement.Debug1",
    )
    .await
    .expect("proxy");
    let answers: Vec<String> = proxy
        .call("Inject", &(vec!["move 1 2", "sleep 5"],))
        .await
        .expect("Inject");
    assert_eq!(answers.len(), 2);
    let (outcome, scene): (String, String) = proxy
        .call("Wait", &("settled", 100_u32))
        .await
        .expect("Wait");
    assert_eq!((outcome.as_str(), scene.as_str()), ("met", "{}"));
    let listed: String = proxy
        .call("Output", &("list", Vec::<String>::new()))
        .await
        .expect("Output");
    assert_eq!(listed, "[]");
    proxy
        .call::<_, _, ()>("Clock", &("step", 2_u32))
        .await
        .expect("Clock");
    let windows: String = proxy.call("Windows", &()).await.expect("Windows");
    assert_eq!(windows, "{}");
    let refused = proxy.call::<_, _, ()>("Clock", &("rewind", 1_u32)).await;
    assert!(refused.is_err());
}
