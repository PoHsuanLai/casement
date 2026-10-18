//! `session_lock` (P0-H3).

use casement::lock_marker::Marked;
use casement_core::OutputName;
use casement_core::geometry::PhysicalSize;
use casement_core::input::{DeviceClass, Edge, KeyCode};
use casement_core::lock::{LockHolder, LockState};
use casement_core::policy::ConnectionRole;
use casement_testclient::seat::keyboard::Keyboard;
use casement_testclient::surface::layer::{
    Anchor, KeyboardInteractivity, Layer, LayerAsk, LayerSurface,
};
use casement_testclient::surface::lock::{LockSurface, Locker};
use casement_testclient::{Pattern, TestClient, shm_buffer};
use casement_testkit::fakes::MemoryLockMarker;
use casement_testkit::{Fixture, FixtureSpec, FrameCount, ShotTarget, TestOrigin};
use std::time::Duration;

use crate::protocol::consent::{asked_world, kept, program};
use crate::protocol::xdg_shell::{map_toplevel, settle};
use casement_core::Event;
use casement_core::privilege::{UnixSeconds, Verdict};
use casement_protocols::vocab::{Answer, Power};
use casement_testclient::surface::toplevel::WmBase;
use casement_testkit::ASKED_PROGRAM;

/// An opaque black pixel.
const BLACK: [u8; 4] = [0, 0, 0, 255];

/// How far from where it rests the cursor reaches.
const CURSOR_REACH: u32 = 32;

/// Where the cursor rests: the centre of the 1920x1080 output, where the pointer starts.
const CURSOR_AT: (u32, u32) = (960, 540);

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
    let shell = fixture.connect(ConnectionRole::Shell);
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

/// A locker that never makes a lock surface is not waited for forever (ext-session-lock: the
/// compositor "must impose a reasonable time limit"): the outputs are under the cover, and once
/// the time is up `locked` arrives.
#[test]
fn a_locker_with_no_lock_surface_is_locked_when_the_wait_runs_out() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let locker = Locker::request(fixture.client(client)).expect("locker");
    fixture.pump();
    assert!(!locker.is_locked(), "it waits for a lock surface at first");
    fixture.advance(Duration::from_millis(500));
    assert!(!locker.is_locked(), "not before the limit");
    fixture.advance(Duration::from_secs(1));
    assert!(locker.is_locked(), "the limit passed with no surface");
}

/// Killing the locker keeps every output covered; nothing of the session is drawn.
#[test]
fn a_dead_locker_leaves_the_lock_cover() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let app = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let base = WmBase::bind(fixture.client(app), 6);
    let _window = map_toplevel(&mut fixture, app, &base, (400, 300));
    settle(&mut fixture);
    // Before the lock the window is on the output: some pixel is not black.
    let output = OutputName::parse("HEADLESS-1").expect("a valid output name");
    fixture.frames(&output, FrameCount(2));
    let before = fixture.shot(ShotTarget::Output(output.clone()));
    assert!(
        before.pixels().any(|pixel| pixel.0 != BLACK),
        "the window is drawn before the lock"
    );

    // A locker covers the output, then dies.
    let owner = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let locker = Locker::request(fixture.client(owner)).expect("a lock");
    fixture.pump();
    let cover = locker.cover(fixture.client(owner)).expect("a lock surface");
    fixture.pump();
    assert!(cover.ack(fixture.client(owner)));
    let size = PhysicalSize { w: 1920, h: 1080 };
    let buffer = shm_buffer(fixture.client(owner), size, Pattern::CoordinateEncoding);
    cover.show(fixture.client(owner), &buffer);
    fixture.pump();
    assert!(locker.is_locked(), "the session is locked");
    fixture.disconnect(owner);
    fixture.pump();
    fixture.advance(Duration::from_secs(2));
    fixture.frames(&output, FrameCount(2));

    // The compositor's own cover stands in: the session is still locked, and nothing of the
    // window (or of the dead locker's lock surface) is drawn.
    assert_eq!(
        fixture.model().windows().len(),
        1,
        "the window is still there, behind the cover"
    );
    let shot = fixture.shot(ShotTarget::Output(output));
    let leaked = shot
        .enumerate_pixels()
        .filter(|(x, y, _)| {
            x.abs_diff(CURSOR_AT.0) >= CURSOR_REACH || y.abs_diff(CURSOR_AT.1) >= CURSOR_REACH
        })
        .filter(|(_, _, pixel)| pixel.0 != BLACK)
        .count();
    assert_eq!(leaked, 0, "the cover left {leaked} pixels showing");
}

/// `id` locks the session with a lock surface that shows content on the first output, and the
/// session is locked when this returns.
fn lock_with_content(fixture: &mut Fixture, id: casement_testkit::ClientId) -> Locker {
    let locker = Locker::request(fixture.client(id)).expect("a lock");
    fixture.pump();
    let cover = locker.cover(fixture.client(id)).expect("a lock surface");
    fixture.pump();
    assert!(cover.ack(fixture.client(id)));
    let size = PhysicalSize { w: 1920, h: 1080 };
    let buffer = shm_buffer(fixture.client(id), size, Pattern::Solid(0xff00_0000));
    cover.show(fixture.client(id), &buffer);
    fixture.pump();
    assert!(locker.is_locked(), "the session is locked");
    locker
}

/// CASC-05: a locker that dies before `locked` never locked anything: the session is open again
/// and the stale wait changes nothing.
#[test]
fn a_locker_that_dies_before_locked_leaves_the_session_open() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let locker = Locker::request(fixture.client(client)).expect("a lock");
    fixture.pump();
    assert!(!locker.is_locked(), "it waits for a lock surface");
    assert!(matches!(fixture.model().lock(), LockState::Locking { .. }));
    fixture.disconnect(client);
    fixture.pump();
    assert_eq!(fixture.model().lock(), &LockState::Unlocked);
    fixture.advance(Duration::from_secs(2));
    assert_eq!(fixture.model().lock(), &LockState::Unlocked);
}

/// CASW-02: once the locker is gone, another public client that asks to lock is refused and
/// the session stays covered; the shell's own locker takes the lock over.
#[test]
fn a_lock_whose_locker_died_is_taken_over_only_by_the_shell() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let owner = fixture.connect(ConnectionRole::Public);
    let hostile = fixture.connect(ConnectionRole::Public);
    let shell = fixture.connect(ConnectionRole::Shell);
    fixture.pump();
    let _locker = lock_with_content(&mut fixture, owner);
    fixture.disconnect(owner);
    fixture.advance(Duration::from_secs(2));
    let orphaned = fixture.model().lock().clone();
    assert!(matches!(orphaned, LockState::LockedWithoutSurface { .. }));

    let grab = Locker::request(fixture.client(hostile)).expect("a lock request");
    fixture.pump();
    assert!(grab.is_finished(), "the newcomer is told the lock is over");
    assert!(!grab.is_locked(), "and does not hold the session");
    assert_eq!(
        fixture.model().lock(),
        &orphaned,
        "the session stays covered, held by the same client"
    );

    let own = Locker::request(fixture.client(shell)).expect("the shell's lock");
    fixture.pump();
    assert!(own.is_locked(), "the shell takes the lock over");
    assert!(!own.is_finished());
}

/// Two 1920x1080 outputs side by side.
const TWO: [&str; 2] = [
    "HEADLESS-1:1920x1080@60000:120:0,0",
    "HEADLESS-2:1920x1080@60000:120:1920,0",
];

/// A window is open on the first of two outputs, `id` locks both with lock surfaces showing
/// content, and the session is locked when this returns: the locker and its two surfaces.
fn lock_both_outputs(
    fixture: &mut Fixture,
    id: casement_testkit::ClientId,
) -> (Locker, LockSurface, LockSurface) {
    let locker = Locker::request(fixture.client(id)).expect("a lock");
    fixture.pump();
    let first = locker
        .cover_on(fixture.client(id), 0)
        .expect("first surface");
    let second = locker
        .cover_on(fixture.client(id), 1)
        .expect("second surface");
    fixture.pump();
    let size = PhysicalSize { w: 1920, h: 1080 };
    for surface in [&first, &second] {
        assert!(surface.ack(fixture.client(id)));
        let buffer = shm_buffer(fixture.client(id), size, Pattern::Solid(0xff00_0000));
        surface.show(fixture.client(id), &buffer);
    }
    fixture.pump();
    assert!(locker.is_locked(), "the session is locked");
    (locker, first, second)
}

/// How many pixels of the first output show something, away from where the cursor rests.
fn showing_on_first(fixture: &mut Fixture) -> usize {
    let output = OutputName::parse("HEADLESS-1").expect("a valid output name");
    fixture.frames(&output, FrameCount(2));
    fixture
        .shot(ShotTarget::Output(output))
        .enumerate_pixels()
        .filter(|(x, y, _)| {
            x.abs_diff(CURSOR_AT.0) >= CURSOR_REACH || y.abs_diff(CURSOR_AT.1) >= CURSOR_REACH
        })
        .filter(|(_, _, pixel)| pixel.0 != BLACK)
        .count()
}

/// CASC-06, CASW-12: a live locker that destroys the lock surface of one output must not
/// reveal the desktop there: the cover goes back up.
#[test]
fn a_destroyed_lock_surface_leaves_its_output_covered() {
    let mut fixture = Fixture::start(FixtureSpec::desktop().with_outputs(&TWO)).expect("fixture");
    let app = fixture.connect(ConnectionRole::Public);
    let owner = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let base = WmBase::bind(fixture.client(app), 6);
    let _window = map_toplevel(&mut fixture, app, &base, (400, 300));
    settle(&mut fixture);
    let (_locker, first, _second) = lock_both_outputs(&mut fixture, owner);
    assert_eq!(showing_on_first(&mut fixture), 0, "covered while locked");

    first.destroy(fixture.client(owner));
    fixture.pump();
    assert_eq!(
        showing_on_first(&mut fixture),
        0,
        "the output whose lock surface went is still covered"
    );
    assert!(matches!(fixture.model().lock(), LockState::Locked { .. }));
}

/// CASC-06: the same when the locker removes the buffer of its lock surface.
#[test]
fn an_unmapped_lock_surface_leaves_its_output_covered() {
    let mut fixture = Fixture::start(FixtureSpec::desktop().with_outputs(&TWO)).expect("fixture");
    let app = fixture.connect(ConnectionRole::Public);
    let owner = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let base = WmBase::bind(fixture.client(app), 6);
    let _window = map_toplevel(&mut fixture, app, &base, (400, 300));
    settle(&mut fixture);
    let (_locker, first, _second) = lock_both_outputs(&mut fixture, owner);

    first.unmap(fixture.client(owner));
    fixture.pump();
    assert_eq!(
        showing_on_first(&mut fixture),
        0,
        "the output whose lock surface showed nothing is still covered"
    );
}

/// CASB-06: casement restarting while the shell's lock is held comes back covered, refuses any
/// client but the shell as the new locker, and is open again only when the shell locks and
/// unlocks. The marker follows: written once the shell's lock is held, removed at the unlock.
#[test]
fn a_restart_while_locked_comes_back_covered_until_the_shell_locks_again() {
    let marker = MemoryLockMarker::absent();
    let spec = || FixtureSpec::desktop().with_lock_marker(marker.clone());
    let mut first = Fixture::start(spec()).expect("fixture");
    let shell = first.connect(ConnectionRole::Shell);
    first.pump();
    assert_eq!(marker.is_marked(), Marked::No, "nothing is locked yet");
    let _locker = lock_with_content(&mut first, shell);
    assert_eq!(
        marker.is_marked(),
        Marked::Yes,
        "the shell's lock is remembered"
    );
    drop(first);

    let restored = LockState::LockedWithoutSurface {
        holder: LockHolder::Shell(None),
    };
    let mut second = Fixture::start(spec()).expect("fixture");
    assert_eq!(second.model().lock(), &restored, "it comes back covered");
    let public = second.connect(ConnectionRole::Public);
    let shell = second.connect(ConnectionRole::Shell);
    second.pump();
    let grab = Locker::request(second.client(public)).expect("a lock request");
    second.pump();
    assert!(
        grab.is_finished() && !grab.is_locked(),
        "a public client is refused"
    );
    assert_eq!(second.model().lock(), &restored);

    let own = lock_with_content(&mut second, shell);
    own.unlock(second.client(shell));
    second.pump();
    assert_eq!(second.model().lock(), &LockState::Unlocked);
    assert_eq!(
        marker.is_marked(),
        Marked::No,
        "the unlock removes the marker"
    );
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

/// Covers the first output with a lock surface that shows content, for a locker that was
/// requested already.
fn cover_with_content(fixture: &mut Fixture, id: casement_testkit::ClientId, locker: &Locker) {
    let cover = locker.cover(fixture.client(id)).expect("a lock surface");
    fixture.pump();
    assert!(cover.ack(fixture.client(id)));
    let size = PhysicalSize { w: 1920, h: 1080 };
    let buffer = shm_buffer(fixture.client(id), size, Pattern::Solid(0xff00_0000));
    cover.show(fixture.client(id), &buffer);
    fixture.pump();
}

/// Locking is a power: a program that is asked gets neither `locked` nor `finished` while the
/// question is open (and the session stays open however long the wait), the shell is asked once,
/// and on Allow the normal lock flow goes on.
#[test]
fn a_lock_waits_for_the_answer_and_goes_on_when_allowed() {
    let (mut fixture, shell, client) = asked_world();
    let locker = Locker::request(fixture.client(client)).expect("a lock");
    fixture.pump();
    assert_eq!(shell.asked().len(), 1, "the shell is asked once");
    assert_eq!(shell.asked()[0].power, Power::Lock);
    fixture.advance(Duration::from_secs(2));
    assert!(!locker.is_locked() && !locker.is_finished(), "it waits");
    assert_eq!(fixture.model().lock(), &LockState::Unlocked);

    shell.answer(&mut fixture, shell.only_ask(), Answer::Allow);
    assert!(matches!(fixture.model().lock(), LockState::Locking { .. }));
    cover_with_content(&mut fixture, client, &locker);
    assert!(locker.is_locked(), "the lock goes on once allowed");
    assert!(!locker.is_finished());
    assert_eq!(
        kept(&fixture, ASKED_PROGRAM, Power::Lock),
        Some(Verdict::Allow)
    );
}

/// Don't Allow: the locker is told `finished` and the session stays open.
#[test]
fn a_refused_lock_is_finished_and_the_session_stays_open() {
    let (mut fixture, shell, client) = asked_world();
    let locker = Locker::request(fixture.client(client)).expect("a lock");
    fixture.pump();
    assert!(!locker.is_finished());
    shell.answer(&mut fixture, shell.only_ask(), Answer::Deny);
    assert!(locker.is_finished(), "the refusal is `finished`");
    assert!(!locker.is_locked());
    assert_eq!(fixture.model().lock(), &LockState::Unlocked);
    assert_eq!(
        kept(&fixture, ASKED_PROGRAM, Power::Lock),
        Some(Verdict::Deny)
    );
}

/// Taking the power back never ends a lock that already holds the session: the covers stay up
/// and the locker can still unlock.
#[test]
fn taking_the_power_back_does_not_end_a_lock_in_force() {
    let (mut fixture, shell, client) = asked_world();
    let locker = Locker::request(fixture.client(client)).expect("a lock");
    fixture.pump();
    shell.answer(&mut fixture, shell.only_ask(), Answer::Allow);
    cover_with_content(&mut fixture, client, &locker);
    assert!(locker.is_locked());

    fixture.feed_events(vec![Event::PrivilegeRevoked {
        subject: program(ASKED_PROGRAM),
        power: Power::Lock,
        at: UnixSeconds(2),
    }]);
    fixture.advance(Duration::from_secs(1));
    assert!(!locker.is_finished(), "the lock is not ended");
    assert!(matches!(fixture.model().lock(), LockState::Locked { .. }));

    locker.unlock(fixture.client(client));
    fixture.pump();
    assert_eq!(
        fixture.model().lock(),
        &LockState::Unlocked,
        "only the locker unlocks, and it still can"
    );
}
