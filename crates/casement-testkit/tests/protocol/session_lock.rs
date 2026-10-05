//! `session_lock` (P0-H3).

use casement_core::geometry::PhysicalSize;
use casement_core::input::{DeviceClass, Edge, KeyCode};
use casement_core::policy::ConnectionRole;
use casement_testclient::seat::keyboard::Keyboard;
use casement_testclient::surface::layer::{
    Anchor, KeyboardInteractivity, Layer, LayerAsk, LayerSurface,
};
use casement_testclient::surface::lock::Locker;
use casement_testclient::{Pattern, TestClient, shm_buffer};
use casement_testkit::{Fixture, FixtureSpec, TestOrigin};
use porter_core::SandboxFacts;

/// Whether the registry the client saw lists `interface`.
fn advertises(client: &mut TestClient, interface: &str) -> bool {
    client
        .globals()
        .contents()
        .clone_list()
        .iter()
        .any(|global| global.interface == interface)
}

/// The evdev code of the `a` key.
const KEY_A: KeyCode = KeyCode(30);

/// How many `key` events `client`'s keyboards received.
fn keys_received(client: &TestClient) -> usize {
    client
        .transcript()
        .0
        .iter()
        .filter(|line| line.object.starts_with("wl_keyboard#") && line.event == "key")
        .count()
}

/// Whether `client`'s keyboard has been told it left its focus.
fn keyboard_left(client: &TestClient) -> bool {
    client
        .transcript()
        .0
        .iter()
        .any(|line| line.object.starts_with("wl_keyboard#") && line.event == "leave")
}

/// While locked, keys and pointer reach only the lock surface.
#[test]
fn lock_blocks_input_to_windows() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let keyboard_device = fixture.input().plug(
        DeviceClass::Keyboard,
        TestOrigin::Physical(DeviceClass::Keyboard),
    );
    // One client holds the keyboard through an exclusive layer, as a launcher does.
    let shell = fixture.connect(ConnectionRole::Public);
    let locker_client = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let _shell_keyboard = Keyboard::bind(fixture.client(shell)).expect("keyboard");
    let _locker_keyboard = Keyboard::bind(fixture.client(locker_client)).expect("keyboard");
    let launcher = LayerSurface::create(
        fixture.client(shell),
        &LayerAsk {
            level: Layer::Overlay,
            namespace: "launcher",
            anchor: Anchor::empty(),
            exclusive_zone: 0,
            keyboard: KeyboardInteractivity::Exclusive,
            size: (400, 300),
        },
    )
    .expect("layer");
    fixture.pump();
    let _ = launcher;
    fixture.input().key(keyboard_device, KEY_A, Edge::Press);
    fixture.input().key(keyboard_device, KEY_A, Edge::Release);
    assert!(
        keys_received(fixture.client(shell)) >= 2,
        "before the lock the launcher has the keys"
    );

    // The lock arrives and covers the output.
    let locker = Locker::request(fixture.client(locker_client)).expect("locker");
    fixture.pump();
    let cover = locker
        .cover(fixture.client(locker_client))
        .expect("lock surface");
    fixture.pump();
    assert!(cover.ack(fixture.client(locker_client)));
    let size = PhysicalSize { w: 1920, h: 1080 };
    let buffer = shm_buffer(
        fixture.client(locker_client),
        size,
        Pattern::Solid(0xff00_0000),
    );
    cover.show(fixture.client(locker_client), &buffer);
    fixture.pump();
    assert!(locker.is_locked());
    assert!(
        keyboard_left(fixture.client(shell)),
        "the launcher lost the keyboard to the lock"
    );

    let before = (
        keys_received(fixture.client(shell)),
        keys_received(fixture.client(locker_client)),
    );
    fixture.input().key(keyboard_device, KEY_A, Edge::Press);
    fixture.input().key(keyboard_device, KEY_A, Edge::Release);
    let after = (
        keys_received(fixture.client(shell)),
        keys_received(fixture.client(locker_client)),
    );
    assert_eq!(
        after.0, before.0,
        "no key reaches the session behind the lock"
    );
    assert!(after.1 > before.1, "the lock surface hears the keys");
}

/// Killing the locker keeps every output covered; nothing of the session is drawn.
#[test]
#[ignore = "body is todo!(): Fixture::disconnect exists now; the cover assertion needs a frame shot (P0-R-scene shoot())"]
fn a_dead_locker_leaves_the_lock_cover() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    let _ = fixture.client(client);
    todo!("P0-H3: killing the locker keeps every output covered; nothing of the session is drawn")
}

/// Locked arrives only after each output shows a lock surface or a cover.
#[test]
fn locked_is_sent_once_every_output_is_covered() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let id = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let locker = Locker::request(fixture.client(id)).expect("locker");
    fixture.pump();
    assert!(
        !locker.is_locked(),
        "nothing is locked before the output is covered"
    );
    assert!(!locker.is_finished());
    let cover = locker.cover(fixture.client(id)).expect("lock surface");
    fixture.pump();
    assert!(
        !locker.is_locked(),
        "a lock surface without content does not cover the output"
    );
    assert!(cover.ack(fixture.client(id)));
    let size = PhysicalSize { w: 1920, h: 1080 };
    let buffer = shm_buffer(fixture.client(id), size, Pattern::Solid(0xff00_0000));
    cover.show(fixture.client(id), &buffer);
    fixture.pump();
    assert!(locker.is_locked(), "locked arrives with the first content");
}

/// A Sandboxed connection does not see ext_session_lock_manager_v1 (I7).
#[test]
fn session_lock_is_hidden_from_sandboxed_clients() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let sandboxed = fixture.connect(ConnectionRole::Sandboxed(SandboxFacts::Flatpak {
        app: "org.example.App".to_owned(),
        instance: "1".to_owned(),
    }));
    let public = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    assert!(
        !advertises(fixture.client(sandboxed), "ext_session_lock_manager_v1"),
        "a sandboxed client must not see ext_session_lock_manager_v1"
    );
    assert!(
        advertises(fixture.client(public), "ext_session_lock_manager_v1"),
        "a public client sees ext_session_lock_manager_v1"
    );
}
