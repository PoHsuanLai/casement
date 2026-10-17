//! `serve_compositor_at` over a private dbus-daemon, with every caller made to stand as a chosen
//! program (`BusPeers::introduce`): a call reaches the event loop's channel only from the caller
//! that holds the role the method answers, all the way; the rest are refused on the bus thread
//! with the one sentence. A stand-in loop answers what gets through. No test reads the real
//! `/proc` for a caller.

use crate::dbus::{CALLS_PER_WINDOW, CONNECT_QUEUE, NOT_THE_CALLER};
use crate::test_served::{BAD_ARGS, DENIED, Loop, Served, TOO_MANY, WHOLE_TEST, outcome, service};
use casement_core::policy::ConnectionRole;
use peephole_bus::peephole::fake::{introduced_program, introduced_unverified};
use peephole_bus::peephole_core::{Proof, WhyNot};
use std::time::Duration;

async fn connect_as(proxy: &zbus::Proxy<'_>, role: &str) -> String {
    outcome(
        proxy
            .call::<_, _, zbus::zvariant::OwnedFd>("Connect", &(role,))
            .await,
    )
}

/// CASW-05: a `Connect` reaches the loop only from the caller that holds the role it asks for,
/// proven all the way. The refusals are the attacks of the caller-identity design: a user-level
/// unit override or a drop-in (the unit manager disagrees, so the proof stops at the image), a
/// copied binary, a moved control group running a user-writable image, a peer that left.
#[tokio::test]
async fn connect_is_decided_by_who_the_caller_proves_to_be() {
    let served = Served::start(WHOLE_TEST, Loop::Reads, None);
    let program = || introduced_program("/usr/bin/sill").expect("a standing");
    // (case, how the caller stands, the role named, what comes back)
    let cases = [
        (
            "the shell, proven all the way",
            service("shell", Proof::Full),
            "shell",
            "granted",
        ),
        (
            "the agent tier, proven all the way",
            service("agent_tier", Proof::Full),
            "agent",
            "granted",
        ),
        (
            "the shell asks for the agent tier",
            service("shell", Proof::Full),
            "agent",
            DENIED,
        ),
        (
            "the agent tier asks for the shell",
            service("agent_tier", Proof::Full),
            "shell",
            DENIED,
        ),
        (
            "the portal asks for the shell",
            service("portal", Proof::Full),
            "shell",
            DENIED,
        ),
        (
            "a user-level unit override or a drop-in: the image is right, the unit is not",
            service("shell", Proof::Exe),
            "shell",
            DENIED,
        ),
        (
            "only the control group names the unit",
            service("shell", Proof::Cgroup),
            "shell",
            DENIED,
        ),
        ("a copied binary no row names", program(), "shell", DENIED),
        (
            "a moved control group running a user-writable image",
            introduced_unverified(WhyNot::ImageWritable),
            "shell",
            DENIED,
        ),
        (
            "a caller that left before it was looked at",
            introduced_unverified(WhyNot::PeerGone),
            "shell",
            DENIED,
        ),
        (
            "a role casement does not hand out",
            service("shell", Proof::Full),
            "public",
            BAD_ARGS,
        ),
    ];
    for (case, standing, role, want) in cases {
        let proxy = served.caller(standing).await;
        assert_eq!(connect_as(&proxy, role).await, want, "{case}");
    }
    let asks = served.asks();
    assert_eq!(asks.len(), 2, "only the two proven callers reach the loop");
    assert_eq!(asks[0].role, ConnectionRole::Shell);
    assert_eq!(asks[0].caller.role.as_str(), "shell");
    assert_eq!(asks[1].role, ConnectionRole::Agent);
    assert_eq!(asks[1].caller.role.as_str(), "agent_tier");
}

/// A refused caller learns one sentence and nothing about which check failed.
#[tokio::test]
async fn a_refusal_says_one_thing_whatever_failed() {
    let served = Served::start(WHOLE_TEST, Loop::Reads, None);
    for standing in [
        service("shell", Proof::Exe),
        introduced_unverified(WhyNot::ImageHidden),
    ] {
        let proxy = served.caller(standing).await;
        let refused = proxy
            .call::<_, _, zbus::zvariant::OwnedFd>("Connect", &("shell",))
            .await;
        match refused {
            Err(zbus::Error::MethodError(_, Some(text), _)) => assert_eq!(text, NOT_THE_CALLER),
            other => panic!("not a refusal with a text: {other:?}"),
        }
    }
}

/// `ConnectEis` and `EndEis` are the portal's alone, proven all the way, and reach the loop as a
/// remote ask and an end ask carrying who asked.
#[tokio::test]
async fn the_remote_input_calls_are_the_portals_alone() {
    let served = Served::start(WHOLE_TEST, Loop::Reads, None);
    let portal = served.caller(service("portal", Proof::Full)).await;
    let shell = served.caller(service("shell", Proof::Full)).await;
    let half = served.caller(service("portal", Proof::Exe)).await;
    let eis = |proxy: zbus::Proxy<'static>, session: u64| async move {
        outcome(
            proxy
                .call::<_, _, zbus::zvariant::OwnedFd>("ConnectEis", &(session, 3_u32))
                .await,
        )
    };
    let end = |proxy: zbus::Proxy<'static>, session: u64| async move {
        match proxy.call::<_, _, ()>("EndEis", &(session,)).await {
            Ok(()) => "ended".to_owned(),
            Err(zbus::Error::MethodError(name, ..)) => name.to_string(),
            Err(other) => format!("{other}"),
        }
    };
    assert_eq!(eis(portal.clone(), 7).await, "granted");
    assert_eq!(eis(shell.clone(), 8).await, DENIED);
    assert_eq!(eis(half.clone(), 9).await, DENIED);
    assert_eq!(end(portal, 7).await, "ended");
    assert_eq!(end(shell, 8).await, DENIED);
    assert_eq!(end(half, 9).await, DENIED);
    // The portal's connection leaving may add a `gone` ask by now (NEW-PI-8, tested in the
    // testkit's remote_eis); this test is about the calls.
    let asks: Vec<_> = served
        .asks()
        .into_iter()
        .filter(|ask| ask.gone.is_none())
        .collect();
    assert_eq!(asks.len(), 2, "{asks:?}");
    assert_eq!(
        asks[0].remote,
        Some(crate::dbus::RemoteAsk {
            session: 7,
            devices: 3
        })
    );
    assert_eq!(asks[0].caller.role.as_str(), "portal");
    assert_eq!(asks[1].end, Some(crate::dbus::EndAsk { session: 7 }));
}

/// One caller gets `CALLS_PER_WINDOW` calls in a window; the rest are told to wait and never
/// reach the loop, proven caller or not.
#[tokio::test]
async fn a_caller_that_asks_too_often_is_told_to_wait() {
    let served = Served::start(WHOLE_TEST, Loop::Reads, None);
    let proxy = served.caller(service("shell", Proof::Full)).await;
    let mut seen = Vec::new();
    for _ in 0..(CALLS_PER_WINDOW + 2) {
        seen.push(connect_as(&proxy, "shell").await);
    }
    let granted = usize::try_from(CALLS_PER_WINDOW).expect("small");
    assert!(seen[..granted].iter().all(|o| o == "granted"), "{seen:?}");
    assert!(seen[granted..].iter().all(|o| o == TOO_MANY), "{seen:?}");
    assert_eq!(served.asks().len(), granted);
}

/// The queue into the loop holds `CONNECT_QUEUE` calls and drops the rest at once, whatever the
/// loop is doing: nine calls from three callers, a loop that reads nothing, eight wait and one is
/// dropped.
#[tokio::test]
async fn a_stalled_loop_holds_the_queue_and_drops_the_rest() {
    let served = Served::start(WHOLE_TEST, Loop::Stalled, None);
    let mut tasks = Vec::new();
    for _ in 0..3 {
        let proxy = served.caller(service("shell", Proof::Full)).await;
        for _ in 0..3 {
            let proxy = proxy.clone();
            tasks.push(tokio::spawn(async move {
                let call = proxy.call::<_, _, zbus::zvariant::OwnedFd>("Connect", &("shell",));
                match tokio::time::timeout(Duration::from_millis(1500), call).await {
                    Ok(result) => outcome(result),
                    Err(_) => "waiting".to_owned(),
                }
            }));
        }
    }
    let mut waiting = 0;
    let mut dropped = 0;
    for task in tasks {
        match task.await.expect("task").as_str() {
            "waiting" => waiting += 1,
            TOO_MANY => dropped += 1,
            other => panic!("unexpected: {other}"),
        }
    }
    assert_eq!((waiting, dropped), (CONNECT_QUEUE, 9 - CONNECT_QUEUE));
}

/// NEW-PI-8: when the portal's bus connection goes away, the event loop is asked to end every
/// remote socket that connection asked for; another connection going changes nothing.
#[tokio::test]
async fn the_loop_is_told_when_the_portals_connection_leaves() {
    let served = Served::start(WHOLE_TEST, Loop::Reads, None);
    let portal = served.caller(service("portal", Proof::Full)).await;
    let other = served.caller(service("portal", Proof::Full)).await;
    let granted = portal
        .call::<_, _, zbus::zvariant::OwnedFd>("ConnectEis", &(7_u64, 3_u32))
        .await;
    assert_eq!(outcome(granted), "granted");
    drop(other);
    let connection = portal.connection().clone();
    let owner = connection
        .unique_name()
        .map(|name| name.as_str().to_owned())
        .expect("a unique name");
    drop(portal);
    connection.close().await.expect("the connection closes");
    let mut gone = None;
    for _ in 0..100 {
        gone = served.asks().into_iter().find_map(|ask| ask.gone);
        if gone.is_some() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert_eq!(gone.map(|gone| gone.owner), Some(owner));
}
