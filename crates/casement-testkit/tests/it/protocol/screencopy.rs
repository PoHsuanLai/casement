//! `screencopy` (wave 2026-10-15, capture lane): `zwlr_screencopy_manager_v1`, the protocol
//! `grim` speaks. A frame of a headless output returns the pixels scanout shows; a region
//! (`grim -g`) is cut from them at the output's scale. The client side is
//! `casement_testclient::capture::ScreenCopy`.

use super::consent::{ScriptedShell, kept};
use casement_core::geometry::LogicalPoint;
use casement_core::input::DeviceClass;
use casement_core::policy::ConnectionRole;
use casement_core::privilege::{UnixSeconds, Verdict};
use casement_core::{DeviceId, Event, OutputName};
use casement_protocols::vocab::{Answer, Power};
use casement_testclient::Pumped;
use casement_testclient::capture::{
    Copied, CopyArea, CopyFailure, CopyKind, CursorMode, ScreenCopy,
};
use casement_testkit::{
    ASKED_PROGRAM, ClientId, Fixture, FixtureSpec, FrameCount, ShotTarget, TestOrigin,
};

fn name(text: &str) -> OutputName {
    OutputName::parse(text).expect("a name")
}

/// One 640x480 output at `scale` (120ths), so a frame is a few hundred kilobytes.
fn output_at(scale: u32) -> FixtureSpec {
    let output = format!("HEADLESS-1:640x480@60000:{scale}:0,0");
    FixtureSpec::desktop().with_outputs(&[output.as_str()])
}

/// Pumps the compositor and the copy until neither moves.
fn settle(fixture: &mut Fixture, copy: &mut ScreenCopy) {
    for _ in 0..100 {
        let compositor = fixture.pump();
        let client = copy.pump().expect("pump the copy");
        if (compositor, client) == (Pumped::Quiet, Pumped::Quiet) {
            return;
        }
    }
    panic!("the compositor and the copy did not go quiet");
}

/// Moves the pointer of `device` to `at` (logical pixels).
fn move_pointer(fixture: &mut Fixture, device: DeviceId, at: (i32, i32)) {
    fixture
        .input()
        .pointer_to(device, LogicalPoint { x: at.0, y: at.1 });
}

/// A fixture with a pointer at (300, 200), a client and two presented frames, the manager bound
/// and the outputs named.
fn started(spec: FixtureSpec) -> (Fixture, ClientId, ScreenCopy, DeviceId) {
    let mut fixture = Fixture::start(spec).expect("fixture");
    let pointer = fixture.input().plug(
        DeviceClass::Pointer,
        TestOrigin::Physical(DeviceClass::Pointer),
    );
    move_pointer(&mut fixture, pointer, (300, 200));
    let client = fixture.connect(ConnectionRole::Public);
    fixture.frames(&name("HEADLESS-1"), FrameCount(2));
    let mut copy = ScreenCopy::start(fixture.client(client).globals()).expect("the manager");
    settle(&mut fixture, &mut copy);
    (fixture, client, copy, pointer)
}

/// Asks for `area` of `HEADLESS-1`, with the cursor in it or not, copies into the first buffer
/// offered and returns the answer.
fn copied(
    fixture: &mut Fixture,
    copy: &mut ScreenCopy,
    area: CopyArea,
    kind: CopyKind,
    cursor: CursorMode,
) -> Result<Copied, CopyFailure> {
    copy.capture("HEADLESS-1", area, cursor).expect("a frame");
    settle(fixture, copy);
    let offer = *copy.offers().first().expect("the frame offered a buffer");
    copy.copy(offer, kind).expect("a copy");
    settle(fixture, copy);
    copy.take().expect("the copy was answered")
}

/// The rows `x..x+w`, `y..y+h` of an RGBA image `width` pixels across.
fn crop(rgba: &[u8], width: u32, (x, y, w, h): (u32, u32, u32, u32)) -> Vec<u8> {
    let at = |n: u32| usize::try_from(n).expect("a small number");
    (y..y + h)
        .flat_map(|row| {
            let from = (at(row) * at(width) + at(x)) * 4;
            rgba[from..from + at(w) * 4].to_vec()
        })
        .collect()
}

/// A public client is offered the manager, a sandbox is not (`sandbox.rs` holds the table).
#[test]
fn a_public_client_is_offered_screencopy() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let public = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let sees = fixture
        .client(public)
        .globals()
        .contents()
        .with_list(|list| {
            list.iter().any(|global| {
                global.interface == "zwlr_screencopy_manager_v1" && global.version == 3
            })
        });
    assert!(sees, "the public socket sees version 3");
}

/// `grim out.png` and `grim -o HEADLESS-1`: the whole output equals the scanout frame.
#[test]
fn a_whole_output_equals_the_scanout_frame() {
    let (mut fixture, _client, mut copy, _pointer) = started(output_at(120));
    assert_eq!(copy.output_names(), ["HEADLESS-1"]);
    let shot = copied(
        &mut fixture,
        &mut copy,
        CopyArea::Output,
        CopyKind::Plain,
        CursorMode::Painted,
    )
    .expect("a frame, not a failure");
    assert_eq!((shot.size.w, shot.size.h), (640, 480));
    let scanout = fixture.shot(ShotTarget::Output(name("HEADLESS-1")));
    assert_eq!(shot.to_rgba8(), scanout.into_raw(), "copy equals scanout");
    assert_eq!(shot.damage, None, "a plain copy reports no damage");
}

/// `grim -g "x,y wxh"`: a region is cut from the frame.
#[test]
fn a_region_crops_the_frame() {
    let (mut fixture, _client, mut copy, _pointer) = started(output_at(120));
    let region = (100, 50, 200, 100);
    let shot = copied(
        &mut fixture,
        &mut copy,
        CopyArea::Region(region),
        CopyKind::Plain,
        CursorMode::Painted,
    )
    .expect("a frame, not a failure");
    assert_eq!((shot.size.w, shot.size.h), (200, 100));
    let scanout = fixture.shot(ShotTarget::Output(name("HEADLESS-1")));
    assert_eq!(
        shot.to_rgba8(),
        crop(&scanout.into_raw(), 640, (100, 50, 200, 100)),
        "the region is the same rows and columns of the scanout frame"
    );
}

/// A region is in logical pixels: at 2x the frame it cuts is twice as large.
#[test]
fn a_region_is_in_logical_pixels() {
    let (mut fixture, _client, mut copy, _pointer) = started(output_at(240));
    let shot = copied(
        &mut fixture,
        &mut copy,
        CopyArea::Region((10, 10, 50, 40)),
        CopyKind::Plain,
        CursorMode::Painted,
    )
    .expect("a frame, not a failure");
    assert_eq!((shot.size.w, shot.size.h), (100, 80));
    let scanout = fixture.shot(ShotTarget::Output(name("HEADLESS-1")));
    assert_eq!(
        shot.to_rgba8(),
        crop(&scanout.into_raw(), 640, (20, 20, 100, 80))
    );
}

/// A region with nothing of the output in it is `failed` at once.
#[test]
fn a_region_off_the_output_fails() {
    let (mut fixture, _client, mut copy, _pointer) = started(output_at(120));
    copy.capture(
        "HEADLESS-1",
        CopyArea::Region((2_000, 2_000, 10, 10)),
        CursorMode::Hidden,
    )
    .expect("a frame");
    settle(&mut fixture, &mut copy);
    assert_eq!(
        copy.take().expect("answered").expect_err("a failure"),
        CopyFailure::Failed
    );
}

/// `copy_with_damage` answers its first frame at once and reports the whole area.
#[test]
fn copy_with_damage_reports_the_whole_area_first() {
    let (mut fixture, _client, mut copy, _pointer) = started(output_at(120));
    let shot = copied(
        &mut fixture,
        &mut copy,
        CopyArea::Output,
        CopyKind::WithDamage,
        CursorMode::Painted,
    )
    .expect("a frame, not a failure");
    assert_eq!(shot.damage, Some(vec![(0, 0, 640, 480)]));
}

/// An unplugged output fails the frame a client was about to ask for.
#[test]
fn an_unplugged_output_is_not_captured() {
    let spec = FixtureSpec::desktop().with_outputs(&[
        "HEADLESS-1:640x480@60000:120:0,0",
        "HEADLESS-2:640x480@60000:120:640,0",
    ]);
    let (mut fixture, _client, mut copy, _pointer) = started(spec);
    fixture.unplug(&name("HEADLESS-2"));
    settle(&mut fixture, &mut copy);
    let kept = copied(
        &mut fixture,
        &mut copy,
        CopyArea::Output,
        CopyKind::Plain,
        CursorMode::Hidden,
    );
    assert!(kept.is_ok(), "the output that stayed is still captured");
}

/// Whether two RGBA images of `width` pixels agree everywhere outside `skip`
/// (`x0, y0, x1, y1`, the right and bottom edges excluded).
fn agree_outside(a: &[u8], b: &[u8], width: usize, skip: (usize, usize, usize, usize)) -> bool {
    a.chunks(4).zip(b.chunks(4)).enumerate().all(|(n, (p, q))| {
        let (x, y) = (n % width, n / width);
        let inside = x >= skip.0 && x < skip.2 && y >= skip.1 && y < skip.3;
        inside || p == q
    })
}

/// `grim -c` puts the cursor in the frame (`overlay_cursor` 1), plain `grim` leaves it out.
#[test]
fn the_cursor_is_in_a_copy_only_when_asked_for() {
    // The arrow's canvas is 24 pixels and its hotspot (5, 3), at (300, 200): 295..319 x 197..221.
    let cursor_box = (290, 190, 325, 225);
    let (mut fixture, _client, mut copy, _pointer) = started(output_at(120));
    let scanout = fixture
        .shot(ShotTarget::Output(name("HEADLESS-1")))
        .into_raw();
    let with = copied(
        &mut fixture,
        &mut copy,
        CopyArea::Output,
        CopyKind::Plain,
        CursorMode::Painted,
    )
    .expect("a frame");
    assert_eq!(
        with.to_rgba8(),
        scanout,
        "overlay_cursor 1: the scanout frame"
    );
    let without = copied(
        &mut fixture,
        &mut copy,
        CopyArea::Output,
        CopyKind::Plain,
        CursorMode::Hidden,
    )
    .expect("a frame");
    assert_ne!(without.to_rgba8(), scanout, "overlay_cursor 0: no cursor");
    assert!(
        agree_outside(&without.to_rgba8(), &scanout, 640, cursor_box),
        "only the cursor's pixels differ"
    );
}

/// cas-wave "Real-run checks: Capture" (`grim` returns within a frame or two): a capture and its
/// copy are answered with at most two frames of the headless clock passing. (The real tty's
/// frame time is hardware and stays a manual check.)
#[test]
fn a_capture_request_completes_within_two_frames() {
    let (mut fixture, _client, mut copy, _pointer) = started(output_at(120));
    let output = name("HEADLESS-1");
    copy.capture("HEADLESS-1", CopyArea::Output, CursorMode::Painted)
        .expect("a frame");
    settle(&mut fixture, &mut copy);
    let offer = *copy.offers().first().expect("the frame offered a buffer");
    copy.copy(offer, CopyKind::Plain).expect("a copy");
    settle(&mut fixture, &mut copy);
    let mut waited = 0;
    let answer = loop {
        if let Some(answer) = copy.take() {
            break answer;
        }
        assert!(waited < 2, "not answered within two frames");
        fixture.frames(&output, FrameCount(1));
        settle(&mut fixture, &mut copy);
        waited += 1;
    };
    assert!(
        answer.is_ok(),
        "the copy failed: {:?}",
        answer.as_ref().err()
    );
}

/// `copy_with_damage` answers its first frame whole, then waits for a change and reports where
/// it was.
#[test]
fn copy_with_damage_reports_what_changed_since_the_last_frame() {
    let (mut fixture, _client, mut copy, pointer) = started(output_at(120));
    let first = copied(
        &mut fixture,
        &mut copy,
        CopyArea::Output,
        CopyKind::WithDamage,
        CursorMode::Painted,
    )
    .expect("a frame");
    assert_eq!(first.damage, Some(vec![(0, 0, 640, 480)]));

    // Nothing moves: frames pass and the copy stays queued.
    copy.capture("HEADLESS-1", CopyArea::Output, CursorMode::Painted)
        .expect("a frame");
    settle(&mut fixture, &mut copy);
    let offer = *copy.offers().first().expect("an offer");
    copy.copy(offer, CopyKind::WithDamage).expect("a copy");
    fixture.frames(&name("HEADLESS-1"), FrameCount(3));
    settle(&mut fixture, &mut copy);
    assert!(copy.take().is_none(), "no change, no answer");

    // The pointer moves: the answer comes with the old and the new place of the cursor.
    move_pointer(&mut fixture, pointer, (100, 100));
    fixture.frames(&name("HEADLESS-1"), FrameCount(2));
    settle(&mut fixture, &mut copy);
    let second = copy.take().expect("answered").expect("a frame");
    let parts = second.damage.expect("the damage");
    assert!(!parts.is_empty(), "something changed");
    assert!(
        parts.iter().all(|(_, _, w, h)| w * h < 640 * 480 / 4),
        "only small parts changed: {parts:?}"
    );
    assert!(
        parts
            .iter()
            .any(|(x, y, w, h)| (*x..x + w).contains(&100) && (*y..y + h).contains(&100)),
        "the new place of the cursor: {parts:?}"
    );
}

/// The damage of a region copy is in the region's own pixels.
#[test]
fn damage_is_in_the_pixels_of_the_copied_region() {
    let (mut fixture, _client, mut copy, pointer) = started(output_at(120));
    let region = CopyArea::Region((50, 50, 100, 100));
    let first = copied(
        &mut fixture,
        &mut copy,
        region,
        CopyKind::WithDamage,
        CursorMode::Painted,
    )
    .expect("a frame");
    assert_eq!(first.damage, Some(vec![(0, 0, 100, 100)]));
    move_pointer(&mut fixture, pointer, (100, 100));
    fixture.frames(&name("HEADLESS-1"), FrameCount(2));
    let second = copied(
        &mut fixture,
        &mut copy,
        region,
        CopyKind::WithDamage,
        CursorMode::Painted,
    )
    .expect("a frame");
    let parts = second.damage.expect("the damage");
    assert!(
        parts
            .iter()
            .any(|(x, y, w, h)| (*x..x + w).contains(&50) && (*y..y + h).contains(&50)),
        "the cursor at (100, 100) is at (50, 50) of the region: {parts:?}"
    );
}

/// CASW-06: a client may have `MAX_PENDING_COPIES` copies waiting for a frame; past that it is
/// cut with `wl_display.no_memory` (code 2), so a pile of `copy_with_damage` cannot make every
/// presented frame cost a pile of read-backs.
#[test]
fn a_client_with_copies_piled_up_is_cut() {
    let (mut fixture, client, _first, _pointer) = started(output_at(120));
    let mut piled = Vec::new();
    // The first `copy_with_damage` of a client is answered at once (it has had no frame of the
    // output); each after it waits for a newer frame, and no frame is presented here.
    let waiting_cap = casement_wayland::limits::MAX_PENDING_COPIES;
    for _ in 0..waiting_cap + 3 {
        let mut copy = ScreenCopy::start(fixture.client(client).globals()).expect("the manager");
        settle(&mut fixture, &mut copy);
        copy.capture("HEADLESS-1", CopyArea::Output, CursorMode::Painted)
            .expect("a frame");
        settle(&mut fixture, &mut copy);
        let offer = *copy.offers().first().expect("the frame offered a buffer");
        copy.copy(offer, CopyKind::WithDamage).expect("a copy");
        fixture.pump();
        piled.push(copy);
        if !fixture.client(client).is_connected() {
            break;
        }
    }
    assert!(
        piled.len() > waiting_cap,
        "the cut came only past the cap, after {} copies",
        piled.len()
    );
    // The cut closes the connection (the system backend sends no error first).
    assert!(
        !fixture.client(client).is_connected(),
        "the client with copies piled up is cut"
    );
}

/// A fixture with the scripted shell, a client that is a program asked about its powers, two
/// presented frames and the manager bound.
fn asked_started() -> (Fixture, ScriptedShell, ClientId, ScreenCopy) {
    let mut fixture = Fixture::start(output_at(120)).expect("fixture");
    let shell = ScriptedShell::start(&mut fixture);
    let client = fixture.connect_via_ask();
    fixture.frames(&name("HEADLESS-1"), FrameCount(2));
    let mut copy = ScreenCopy::start(fixture.client(client).globals()).expect("the manager");
    settle(&mut fixture, &mut copy);
    (fixture, shell, client, copy)
}

/// Asks for the whole output and copies into the first buffer offered, without waiting for any
/// answer.
fn ask_and_copy(fixture: &mut Fixture, copy: &mut ScreenCopy, kind: CopyKind) {
    copy.capture("HEADLESS-1", CopyArea::Output, CursorMode::Hidden)
        .expect("a frame");
    settle(fixture, copy);
    let offer = *copy.offers().first().expect("the frame offered a buffer");
    copy.copy(offer, kind).expect("a copy");
    settle(fixture, copy);
}

/// A copy asked for by a program that is asked waits: the shell is asked once about the screen,
/// and the frame is copied when the person allows.
#[test]
fn a_copy_waits_for_the_answer_and_is_made_on_allow() {
    let (mut fixture, shell, _client, mut copy) = asked_started();
    ask_and_copy(&mut fixture, &mut copy, CopyKind::Plain);
    assert!(copy.take().is_none(), "nothing is copied before the answer");
    let asked = shell.asked();
    assert_eq!(asked.len(), 1);
    assert_eq!(asked[0].power, Power::Screen);
    assert_eq!(asked[0].subject, ASKED_PROGRAM);

    shell.answer(&mut fixture, shell.only_ask(), Answer::Allow);
    settle(&mut fixture, &mut copy);
    let shot = copy
        .take()
        .expect("answered")
        .expect("a frame, not a failure");
    assert_eq!((shot.size.w, shot.size.h), (640, 480));
    assert_eq!(
        kept(&fixture, ASKED_PROGRAM, Power::Screen),
        Some(Verdict::Allow)
    );
}

/// Don't Allow fails the copy that waited; the refusal is kept, so the next copy fails at once
/// without asking again.
#[test]
fn a_refused_copy_fails_and_is_not_asked_about_again() {
    let (mut fixture, shell, _client, mut copy) = asked_started();
    ask_and_copy(&mut fixture, &mut copy, CopyKind::Plain);
    shell.answer(&mut fixture, shell.only_ask(), Answer::Deny);
    settle(&mut fixture, &mut copy);
    assert_eq!(
        copy.take().expect("answered").expect_err("a failure"),
        CopyFailure::Failed
    );
    assert_eq!(
        kept(&fixture, ASKED_PROGRAM, Power::Screen),
        Some(Verdict::Deny)
    );

    ask_and_copy(&mut fixture, &mut copy, CopyKind::Plain);
    assert_eq!(
        copy.take().expect("answered").expect_err("a failure"),
        CopyFailure::Failed
    );
    assert_eq!(shell.asked().len(), 1, "no second question");
}

/// Taking the power back fails the copy that was queued for a newer frame.
#[test]
fn a_power_taken_back_fails_the_queued_copy() {
    let (mut fixture, shell, _client, mut copy) = asked_started();
    ask_and_copy(&mut fixture, &mut copy, CopyKind::Plain);
    shell.answer(&mut fixture, shell.only_ask(), Answer::Allow);
    settle(&mut fixture, &mut copy);
    copy.take().expect("answered").expect("the first copy");

    // The client has had the frame there is; this one waits for a newer.
    ask_and_copy(&mut fixture, &mut copy, CopyKind::WithDamage);
    assert!(copy.take().is_none(), "queued for a newer frame");
    fixture.feed_events(vec![Event::PrivilegeRevoked {
        subject: super::consent::program(ASKED_PROGRAM),
        power: Power::Screen,
        at: UnixSeconds(2),
    }]);
    settle(&mut fixture, &mut copy);
    assert_eq!(
        copy.take().expect("answered").expect_err("a failure"),
        CopyFailure::Failed
    );
}

/// Copies held for the answer count towards the cap on a client's waiting copies: a program
/// that piles them up while the question is open is cut like one that piles up queued ones.
#[test]
fn a_client_with_copies_held_for_the_answer_is_cut_at_the_cap() {
    let (mut fixture, _shell, client, _first) = asked_started();
    let cap = casement_wayland::limits::MAX_PENDING_COPIES;
    let mut piled = Vec::new();
    for _ in 0..cap + 3 {
        let mut copy = ScreenCopy::start(fixture.client(client).globals()).expect("the manager");
        settle(&mut fixture, &mut copy);
        ask_and_copy(&mut fixture, &mut copy, CopyKind::Plain);
        piled.push(copy);
        if !fixture.client(client).is_connected() {
            break;
        }
    }
    assert!(piled.len() > cap, "the cut came only past the cap");
    assert!(
        !fixture.client(client).is_connected(),
        "the client with held copies piled up is cut"
    );
}
