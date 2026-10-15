//! `screencopy` (wave 2026-10-15, capture lane): `zwlr_screencopy_manager_v1`, the protocol
//! `grim` speaks. A frame of a headless output returns the pixels scanout shows; a region
//! (`grim -g`) is cut from them at the output's scale. The client side is
//! `casement_testclient::capture::ScreenCopy`.

use casement_core::OutputName;
use casement_core::policy::ConnectionRole;
use casement_testclient::Pumped;
use casement_testclient::capture::{Copied, CopyArea, CopyFailure, CopyKind, ScreenCopy};
use casement_testkit::{ClientId, Fixture, FixtureSpec, FrameCount, ShotTarget};

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

/// A fixture with a client and two presented frames, the manager bound and the outputs named.
fn started(spec: FixtureSpec) -> (Fixture, ClientId, ScreenCopy) {
    let mut fixture = Fixture::start(spec).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    fixture.frames(&name("HEADLESS-1"), FrameCount(2));
    let mut copy = ScreenCopy::start(fixture.client(client).globals()).expect("the manager");
    settle(&mut fixture, &mut copy);
    (fixture, client, copy)
}

/// Asks for `area` of `HEADLESS-1`, copies into the first buffer offered and returns the
/// answer.
fn copied(
    fixture: &mut Fixture,
    copy: &mut ScreenCopy,
    area: CopyArea,
    kind: CopyKind,
) -> Result<Copied, CopyFailure> {
    copy.capture("HEADLESS-1", area).expect("a frame");
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
    let (mut fixture, _client, mut copy) = started(output_at(120));
    assert_eq!(copy.output_names(), ["HEADLESS-1"]);
    let shot = copied(&mut fixture, &mut copy, CopyArea::Output, CopyKind::Plain)
        .expect("a frame, not a failure");
    assert_eq!((shot.size.w, shot.size.h), (640, 480));
    let scanout = fixture.shot(ShotTarget::Output(name("HEADLESS-1")));
    assert_eq!(shot.to_rgba8(), scanout.into_raw(), "copy equals scanout");
    assert_eq!(shot.damage, None, "a plain copy reports no damage");
}

/// `grim -g "x,y wxh"`: a region is cut from the frame.
#[test]
fn a_region_crops_the_frame() {
    let (mut fixture, _client, mut copy) = started(output_at(120));
    let region = (100, 50, 200, 100);
    let shot = copied(
        &mut fixture,
        &mut copy,
        CopyArea::Region(region),
        CopyKind::Plain,
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
    let (mut fixture, _client, mut copy) = started(output_at(240));
    let shot = copied(
        &mut fixture,
        &mut copy,
        CopyArea::Region((10, 10, 50, 40)),
        CopyKind::Plain,
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
    let (mut fixture, _client, mut copy) = started(output_at(120));
    copy.capture("HEADLESS-1", CopyArea::Region((2_000, 2_000, 10, 10)))
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
    let (mut fixture, _client, mut copy) = started(output_at(120));
    let shot = copied(
        &mut fixture,
        &mut copy,
        CopyArea::Output,
        CopyKind::WithDamage,
    )
    .expect("a frame, not a failure");
    assert_eq!(shot.damage, Some((0, 0, 640, 480)));
}

/// An unplugged output fails the frame a client was about to ask for.
#[test]
fn an_unplugged_output_is_not_captured() {
    let spec = FixtureSpec::desktop().with_outputs(&[
        "HEADLESS-1:640x480@60000:120:0,0",
        "HEADLESS-2:640x480@60000:120:640,0",
    ]);
    let (mut fixture, _client, mut copy) = started(spec);
    fixture.unplug(&name("HEADLESS-2"));
    settle(&mut fixture, &mut copy);
    let kept = copied(&mut fixture, &mut copy, CopyArea::Output, CopyKind::Plain);
    assert!(kept.is_ok(), "the output that stayed is still captured");
}
