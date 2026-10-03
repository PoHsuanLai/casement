//! The fixture itself (P0-B-headless): quiescence without sleeping, outputs, virtual time.

use casement_backend::VirtualOutputSpec;
use casement_core::OutputName;
use casement_testclient::Pumped;
use casement_testkit::{Fixture, FixtureSpec, FrameCount, ShotTarget};
use std::time::Duration;

fn name(text: &str) -> OutputName {
    OutputName::parse(text).expect("an output name")
}

#[test]
#[ignore = "P0-B-headless: runs once P0-B-loop, P0-F1-output (config::load), P0-H1 (output, presentation) and P0-R-scene (render_output) are merged"]
fn a_started_fixture_is_quiet_and_has_its_output() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    assert_eq!(fixture.pump(), Pumped::Quiet);
    assert_eq!(fixture.model().outputs().0.len(), 1);
}

#[test]
#[ignore = "P0-B-headless: runs once P0-B-loop, P0-F1-output (config::load), P0-H1 (output, presentation) and P0-R-scene (render_output) are merged"]
fn three_outputs_at_one_one_and_a_half_and_two() {
    let spec = FixtureSpec::desktop().with_outputs(&[
        "HEADLESS-1:1920x1080@60000:120:0,0",
        "HEADLESS-2:2880x1620@60000:180:1920,0",
        "HEADLESS-3:3840x2160@60000:240:3840,0",
    ]);
    let fixture = Fixture::start(spec).expect("fixture");
    assert_eq!(fixture.model().outputs().0.len(), 3);
    // Each output keeps the scale and place its spec names, not the policy's first choice.
    let outputs = fixture.model().outputs().0.values();
    let (scales, places): (Vec<u32>, Vec<i32>) = outputs
        .map(|output| (output.config.scale.0, output.config.position.x))
        .unzip();
    assert_eq!(scales, [120, 180, 240]);
    assert_eq!(places, [0, 1920, 3840]);
}

#[test]
#[ignore = "P0-B-headless: runs once P0-B-loop, P0-F1-output (config::load), P0-H1 (output, presentation) and P0-R-scene (render_output) are merged"]
fn a_hundred_plug_unplug_cycles_leak_nothing() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let before = fixture.scene();
    for n in 0..100 {
        let text = format!("HEADLESS-9:1280x720@60000:120:{},0", 1920 + n);
        let plugged = fixture.plug(VirtualOutputSpec::parse(&text).expect("spec"));
        assert_eq!(fixture.model().outputs().0.len(), 2);
        fixture.unplug(&plugged);
        assert_eq!(fixture.model().outputs().0.len(), 1);
    }
    assert_eq!(fixture.pump(), Pumped::Quiet);
    assert_eq!(fixture.scene(), before);
}

#[test]
#[ignore = "P0-B-headless: runs once P0-B-loop, P0-F1-output (config::load), P0-H1 (output, presentation) and P0-R-scene (render_output) are merged"]
fn frames_come_only_with_virtual_time() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    assert_eq!(fixture.advance(Duration::ZERO).frames, 0);
    let advanced = fixture.frames(&name("HEADLESS-1"), FrameCount(3));
    assert_eq!(advanced.frames, 3);
}

#[test]
#[ignore = "P0-B-headless: runs once P0-B-loop, P0-F1-output (config::load), P0-H1 (output, presentation) and P0-R-scene (render_output) are merged"]
fn chopping_an_advance_differently_changes_nothing() {
    // SP3: one 6 s advance, 6000 one-millisecond advances and an irregular split agree.
    let run = |chunks: &[u64]| {
        let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
        let frames: u32 = chunks
            .iter()
            .map(|ms| fixture.advance(Duration::from_millis(*ms)).frames)
            .sum();
        (frames, fixture.scene())
    };
    let whole = run(&[6000]);
    assert_eq!(run(&[1; 6000]), whole);
    assert_eq!(run(&[7, 993, 2500, 1, 1499, 1000]), whole);
}

#[test]
#[ignore = "P0-B-headless: runs once P0-B-loop, P0-F1-output (config::load), P0-H1 (output, presentation) and P0-R-scene (render_output) are merged"]
fn two_fixtures_on_one_thread_keep_their_own_clocks() {
    // SP3: a thread-installed clock gave the first fixture 1 vblank in 1 s while a second was
    // alive; casement's clocks are values.
    let mut first = Fixture::start(FixtureSpec::desktop()).expect("first");
    let mut second = Fixture::start(FixtureSpec::desktop()).expect("second");
    assert_eq!(first.advance(Duration::from_secs(1)).frames, 60);
    assert_eq!(second.advance(Duration::from_secs(1)).frames, 60);
}

#[test]
#[ignore = "P0-B-headless: runs once P0-B-loop, P0-F1-output (config::load), P0-H1 (output, presentation) and P0-R-scene (render_output) are merged"]
fn a_shot_of_an_output_is_its_mode_size() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    fixture.frames(&name("HEADLESS-1"), FrameCount(1));
    let shot = fixture.shot(ShotTarget::Output(name("HEADLESS-1")));
    assert_eq!(shot.dimensions(), (1920, 1080));
}
